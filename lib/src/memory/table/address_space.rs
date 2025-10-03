use crate::{
    component::{ComponentId, ComponentPath},
    machine::registry::ComponentRegistry,
    memory::Address,
};
use bitvec::{field::BitField, order::Lsb0};
use bytes::Bytes;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::{
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    sync::{
        Mutex, RwLock, RwLockReadGuard,
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

#[derive(Debug, Clone)]
pub struct ReadEntry {
    pub id: ComponentId,
    pub buffer: Option<Bytes>,
}

impl PartialEq for ReadEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Eq for ReadEntry {}

#[derive(Default, Debug)]
pub struct Members {
    read: RangeInclusiveMap<Address, ReadEntry>,
    write: RangeInclusiveMap<Address, ComponentId>,
}

impl Members {
    #[inline]
    pub fn iter_read(
        &self,
        access_range: RangeInclusive<Address>,
    ) -> impl Iterator<Item = (&RangeInclusive<Address>, &ReadEntry)> {
        self.read.overlapping(access_range.clone())
    }

    #[inline]
    pub fn iter_write(
        &self,
        access_range: RangeInclusive<Address>,
    ) -> impl Iterator<Item = (&RangeInclusive<Address>, &ComponentId)> {
        self.write.overlapping(access_range.clone())
    }
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    pub width: Address,
    members: RwLock<Members>,
    /// Queue for if the address space is locked at the moment
    queue: Mutex<Vec<MemoryRemappingCommands>>,
    queue_modified: AtomicBool,
}

impl AddressSpace {
    pub fn new(address_space_width: u8) -> Self {
        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load_le();

        let width = 2usize.pow(address_space_width as u32);

        Self {
            width_mask,
            width,
            members: Default::default(),
            queue: Default::default(),
            queue_modified: AtomicBool::new(false),
        }
    }

    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommands>) {
        let mut queue_guard = self.queue.lock().unwrap();

        queue_guard.extend(commands);
        self.queue_modified.store(true, Ordering::Release);
    }

    #[inline]
    pub fn get_members(&self, registry: &ComponentRegistry) -> RwLockReadGuard<'_, Members> {
        if self.queue_modified.swap(false, Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.update_members(queue_guard.drain(..), registry);
        }

        self.members.read().unwrap()
    }

    #[cold]
    fn update_members(
        &self,
        commands: impl IntoIterator<Item = MemoryRemappingCommands>,
        registry: &ComponentRegistry,
    ) {
        let invalid_ranges = (0..self.width).complement();
        let mut members = self.members.write().unwrap();

        for command in commands {
            match command {
                MemoryRemappingCommands::RemoveRead { range }
                | MemoryRemappingCommands::RemoveWrite { range } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range,
                            self.width - 1
                        );
                    }

                    tracing::debug!("Removing memory range {:#04x?} from address space", range,);

                    members.read.remove(range.clone());
                }
                MemoryRemappingCommands::MapReadComponent {
                    range,
                    path,
                    buffer,
                } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?} (inserted by {})",
                            range,
                            self.width - 1,
                            path
                        );
                    }

                    if let Some(buffer) = buffer.as_ref() {
                        assert_eq!(
                            buffer.len(),
                            range.clone().count(),
                            "Buffer does not represent the mapped range"
                        );
                    }

                    tracing::debug!(
                        "Mapping read memory to address range {:#04x?} for {}",
                        range,
                        path
                    );

                    let id = registry.get_id(&path).unwrap();
                    members.read.insert(range.clone(), ReadEntry { id, buffer });
                }
                MemoryRemappingCommands::MapWriteComponent { range, path } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?} (inserted by {})",
                            range,
                            self.width - 1,
                            path
                        );
                    }

                    tracing::debug!(
                        "Mapping write memory to address range {:#04x?} for {}",
                        range,
                        path
                    );

                    let id = registry.get_id(&path).unwrap();
                    members.write.insert(range.clone(), id);
                }
                MemoryRemappingCommands::MapComponent { range, path } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?} (inserted by {})",
                            range,
                            self.width - 1,
                            path
                        );
                    }

                    tracing::debug!(
                        "Mapping memory to address range {:#04x?} for {}",
                        range,
                        path
                    );

                    let id = registry.get_id(&path).unwrap();

                    members
                        .read
                        .insert(range.clone(), ReadEntry { id, buffer: None });
                    members.write.insert(range.clone(), id);
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum MemoryRemappingCommands {
    MapReadComponent {
        range: RangeInclusive<Address>,
        path: ComponentPath,
        buffer: Option<Bytes>,
    },
    MapWriteComponent {
        range: RangeInclusive<Address>,
        path: ComponentPath,
    },
    MapComponent {
        range: RangeInclusive<Address>,
        path: ComponentPath,
    },
    RemoveRead {
        range: RangeInclusive<Address>,
    },
    RemoveWrite {
        range: RangeInclusive<Address>,
    },
}
