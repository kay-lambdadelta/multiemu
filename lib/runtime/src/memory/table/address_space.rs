use crate::{component::ComponentId, memory::Address};
use arc_swap::ArcSwap;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use std::{
    hash::{Hash, Hasher},
    num::NonZero,
    ops::RangeInclusive,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    vec::Vec,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct AddressSpaceHandle(NonZero<u16>);

impl AddressSpaceHandle {
    pub fn new(id: u16) -> Option<Self> {
        NonZero::new(id).map(AddressSpaceHandle)
    }
}

impl Hash for AddressSpaceHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u16(self.0.get());
    }
}

impl IsEnabled for AddressSpaceHandle {}

#[derive(Default, Debug, Clone)]
pub struct Members {
    read: RangeInclusiveMap<Address, ComponentId>,
    write: RangeInclusiveMap<Address, ComponentId>,
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    members: ArcSwap<Members>,
    /// Queue for if the address space is locked at the moment
    queue: Mutex<Vec<MemoryRemappingCommands>>,
    queue_modified: AtomicBool,
}

impl AddressSpace {
    pub fn new(width_mask: Address) -> Self {
        Self {
            width_mask,
            members: Default::default(),
            queue: Default::default(),
            queue_modified: AtomicBool::new(false),
        }
    }

    /// Removes all memory maps for a component_id and remaps it like so
    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommands>) {
        let mut queue_guard = self.queue.lock().unwrap();
        queue_guard.extend(commands);
        self.queue_modified.store(true, Ordering::Release);
    }

    #[inline]
    pub fn visit_read_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.load(Ordering::Acquire) {
            self.members_rcu_from_queue();
        }

        for (component_assigned_range, component_id) in self
            .members
            .load()
            .read
            .overlapping(accessing_range.clone())
        {
            callback(*component_id, component_assigned_range)?
        }

        Ok(())
    }

    #[inline]
    pub fn visit_write_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.load(Ordering::Acquire) {
            self.members_rcu_from_queue();
        }

        for (component_assigned_range, component_id) in self
            .members
            .load()
            .write
            .overlapping(accessing_range.clone())
        {
            callback(*component_id, component_assigned_range)?
        }

        Ok(())
    }

    fn members_rcu_from_queue(&self) {
        let mut queue_guard = self.queue.lock().unwrap();
        let queue_contents = std::mem::replace(&mut *queue_guard, Vec::new());

        self.members.rcu(|members| {
            let mut members = (**members).clone();

            for command in queue_contents.iter() {
                match command {
                    MemoryRemappingCommands::Remove { range, types } => {
                        tracing::debug!("Removing memory range {:#04x?} from address space", range);

                        if types.contains(&MemoryType::Read) {
                            members.read.remove(range.clone());
                        }

                        if types.contains(&MemoryType::Write) {
                            members.write.remove(range.clone());
                        }
                    }
                    MemoryRemappingCommands::Add {
                        range,
                        component_id,
                        types,
                    } => {
                        tracing::debug!(
                            "Mapping memory component_id {:?} to address range {:#04x?}",
                            component_id,
                            range
                        );

                        if types.contains(&MemoryType::Read) {
                            members.read.insert(range.clone(), *component_id);
                        }

                        if types.contains(&MemoryType::Write) {
                            members.write.insert(range.clone(), *component_id);
                        }
                    }
                }
            }

            self.queue_modified.store(false, Ordering::Release);

            members
        });
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum MemoryType {
    Read,
    Write,
}

#[derive(Debug)]
pub enum MemoryRemappingCommands {
    Add {
        range: RangeInclusive<Address>,
        component_id: ComponentId,
        types: Vec<MemoryType>,
    },
    Remove {
        range: RangeInclusive<Address>,
        types: Vec<MemoryType>,
    },
}
