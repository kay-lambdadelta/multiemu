use crate::{component::ComponentId, memory::Address};
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use std::{
    hash::{Hash, Hasher},
    num::NonZero,
    ops::RangeInclusive,
    sync::{
        Mutex, RwLock,
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

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    read_members: RwLock<RangeInclusiveMap<Address, ComponentId>>,
    write_members: RwLock<RangeInclusiveMap<Address, ComponentId>>,
    /// Queue for if the address space is locked at the moment
    queue: Mutex<Vec<MemoryRemappingCommands>>,
    queue_modified: AtomicBool,
}

impl AddressSpace {
    pub fn new(width_mask: Address) -> Self {
        Self {
            width_mask,
            read_members: Default::default(),
            write_members: Default::default(),
            queue: Default::default(),
            queue_modified: AtomicBool::new(false),
        }
    }

    /// Removes all memory maps for a component_id and remaps it like so
    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommands>) {
        let mut read_members = self.read_members.try_write();
        let mut write_members = self.write_members.try_write();
        let mut queue_guard = self.queue.lock().unwrap();

        for command in commands {
            match command {
                MemoryRemappingCommands::Remove { range, mut types } => {
                    tracing::debug!("Removing memory range {:#04x?} from address space", range);

                    if let Some(index) = types.iter().position(|t| *t == MemoryType::Read) {
                        if let Ok(read_members) = read_members.as_mut() {
                            read_members.remove(range.clone());
                            types.remove(index);
                        }
                    }

                    if let Some(index) = types.iter().position(|t| *t == MemoryType::Write) {
                        if let Ok(write_members) = write_members.as_mut() {
                            write_members.remove(range.clone());
                            types.remove(index);
                        }
                    }

                    if !types.is_empty() {
                        queue_guard.push(MemoryRemappingCommands::Remove { range, types });
                        self.queue_modified.store(true, Ordering::Release);
                    }
                }
                MemoryRemappingCommands::Add {
                    range,
                    component_id,
                    mut types,
                } => {
                    tracing::debug!(
                        "Mapping memory component_id {:?} to address range {:#04x?}",
                        component_id,
                        range
                    );

                    if let Some(index) = types.iter().position(|t| *t == MemoryType::Read) {
                        if let Ok(read_members) = read_members.as_mut() {
                            read_members.insert(range.clone(), component_id);
                            types.remove(index);
                        }
                    }

                    if let Some(index) = types.iter().position(|t| *t == MemoryType::Write) {
                        if let Ok(write_members) = write_members.as_mut() {
                            write_members.insert(range.clone(), component_id);
                            types.remove(index);
                        }
                    }

                    if !types.is_empty() {
                        queue_guard.push(MemoryRemappingCommands::Add {
                            range,
                            component_id,
                            types,
                        });
                        self.queue_modified.store(true, Ordering::Release);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn visit_read_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.load(Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.remap(queue_guard.drain(..));
            self.queue_modified.store(false, Ordering::Release);
        }

        for (component_assigned_range, component_id) in self
            .read_members
            .read()
            .unwrap()
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
            let mut queue_guard = self.queue.lock().unwrap();
            self.remap(queue_guard.drain(..));
            self.queue_modified.store(false, Ordering::Release);
        }

        for (component_assigned_range, component_id) in self
            .write_members
            .read()
            .unwrap()
            .overlapping(accessing_range.clone())
        {
            callback(*component_id, component_assigned_range)?
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum MemoryType {
    Read,
    Write,
}

#[derive(Debug)]
pub enum MemoryRemappingCommands {
    Remove {
        range: RangeInclusive<Address>,
        types: Vec<MemoryType>,
    },
    Add {
        range: RangeInclusive<Address>,
        component_id: ComponentId,
        types: Vec<MemoryType>,
    },
}
