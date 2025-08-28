use crate::{
    component::{ComponentId, ComponentRegistry},
    memory::Address,
};
use bitvec::{field::BitField, order::Lsb0};
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::{
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    sync::{
        Arc, Mutex, RwLock, RwLockWriteGuard,
        atomic::{AtomicBool, Ordering},
    },
    vec::Vec,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct AddressSpaceId(u16);

impl AddressSpaceId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }
}

impl Hash for AddressSpaceId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u16(self.0);
    }
}

impl IsEnabled for AddressSpaceId {}

#[derive(Default, Debug)]
pub struct Members {
    read: RangeInclusiveMap<Address, ComponentId>,
    write: RangeInclusiveMap<Address, ComponentId>,
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    max_value: Address,
    members: RwLock<Members>,
    /// Queue for if the address space is locked at the moment
    queue: Mutex<Vec<MemoryRemappingCommands>>,
    queue_modified: AtomicBool,
    registry: Arc<ComponentRegistry>,
}

impl AddressSpace {
    pub fn new(registry: Arc<ComponentRegistry>, address_space_width: u8) -> Self {
        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load_le();

        let max_value = 2usize.pow(address_space_width as u32) - 1;

        Self {
            width_mask,
            max_value,
            members: Default::default(),
            queue: Default::default(),
            queue_modified: AtomicBool::new(false),
            registry,
        }
    }

    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommands>) {
        let mut queue_guard = self.queue.lock().unwrap();

        if let Ok(members) = self.members.try_write() {
            self.update_members(Some(members), commands);
        } else {
            queue_guard.extend(commands);
            self.queue_modified.store(true, Ordering::Release);
        }
    }

    #[inline]
    pub fn visit_read_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.swap(false, Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.update_members(None, queue_guard.drain(..));
        }

        let members = self.members.read().unwrap();

        for (component_assigned_addresses, component_id) in
            members.read.overlapping(accessing_range)
        {
            callback(*component_id, component_assigned_addresses)?;
        }

        Ok(())
    }

    #[inline]
    pub fn visit_write_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.swap(false, Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.update_members(None, queue_guard.drain(..));
        }

        let members = self.members.read().unwrap();

        for (component_assigned_addresses, component_id) in
            members.write.overlapping(accessing_range)
        {
            callback(*component_id, component_assigned_addresses)?;
        }

        Ok(())
    }

    #[cold]
    fn update_members(
        &self,
        members: Option<RwLockWriteGuard<'_, Members>>,
        commands: impl IntoIterator<Item = MemoryRemappingCommands>,
    ) {
        let invalid_ranges = (0..=self.max_value).complement();

        let mut members = members.unwrap_or_else(|| self.members.write().unwrap());

        for command in commands {
            match command {
                MemoryRemappingCommands::Remove { range, types } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range, self.max_value
                        );
                    }

                    tracing::debug!(
                        "Removing memory range {:#04x?} from address space on {:?}",
                        range,
                        types
                    );

                    if types.contains(&MemoryType::Read) {
                        members.read.remove(range.clone());
                    }

                    if types.contains(&MemoryType::Write) {
                        members.write.remove(range.clone());
                    }
                }
                MemoryRemappingCommands::AddComponent {
                    range,
                    component_id,
                    types,
                } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} (inserted by component {}) is invalid for a address space that ends at {:04x?} on {:?}",
                            range,
                            self.registry.get_path(component_id),
                            self.max_value,
                            types
                        );
                    }

                    tracing::debug!(
                        "Mapping memory component {} to address range {:#04x?}",
                        self.registry.get_path(component_id),
                        range
                    );

                    if types.contains(&MemoryType::Read) {
                        members.read.insert(range.clone(), component_id);
                    }

                    if types.contains(&MemoryType::Write) {
                        members.write.insert(range.clone(), component_id);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum MemoryType {
    Read,
    Write,
}

#[derive(Debug)]
pub enum MemoryRemappingCommands {
    AddComponent {
        range: RangeInclusive<Address>,
        component_id: ComponentId,
        types: Vec<MemoryType>,
    },
    Remove {
        range: RangeInclusive<Address>,
        types: Vec<MemoryType>,
    },
}
