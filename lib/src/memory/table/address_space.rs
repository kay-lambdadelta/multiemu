use crate::{
    component::{ComponentId, ComponentPath},
    machine::registry::ComponentRegistry,
    memory::Address,
};
use bitvec::{field::BitField, order::Lsb0};
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::{
    hash::{Hash, Hasher},
    ops::{Add, RangeInclusive},
    sync::{
        atomic::{AtomicBool, Ordering}, Mutex, RwLock, RwLockReadGuard
    },
    vec::Vec,
};

const SHARD_SIZE: Address = 0x1000;

#[derive(Debug)]
struct SparseTableEntry {
    pub start: Address,
    pub end: Address,
    pub component: ComponentId,
}

#[derive(Debug, Default)]
pub struct MemoryMappingTable {
    master: RangeInclusiveMap<Address, ComponentId>,
    table: Vec<SparseTableEntry>,
}

impl MemoryMappingTable {
    #[inline]
    pub fn overlapping(
        &self,
        access_range: RangeInclusive<Address>,
    ) -> impl Iterator<Item = (RangeInclusive<Address>, ComponentId)> {
        let start = *access_range.start();
        let end = *access_range.end();

        let index = self
            .table
            .binary_search_by(|entry| {
                if entry.end < start {
                    std::cmp::Ordering::Less
                } else if entry.start > start {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .unwrap_or_else(|i| i);

        let left = self.table[..index]
            .iter()
            .rev()
            .take_while(move |entry| entry.end >= start);

        let right = self.table[index..]
            .iter()
            .take_while(move |entry| entry.start <= end);

        left.chain(right)
            .map(|entry| (entry.start..=entry.end, entry.component))
    }

    pub fn insert(&mut self, range: RangeInclusive<Address>, component: ComponentId) {
        self.master.insert(range, component);
    }

    pub fn remove(&mut self, range: RangeInclusive<Address>) {
        self.master.remove(range);
    }

    pub fn commit(&mut self) {
        self.table.clear();

        for (range, component) in self.master.iter() {
            self.table.push(SparseTableEntry {
                start: *range.start(),
                end: *range.end(),
                component: *component,
            });
        }

        self.table.sort_by_key(|entry| entry.start);
    }
}

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
    read: MemoryMappingTable,
    write: MemoryMappingTable,
}

impl Members {
    #[inline]
    pub fn iter_read(
        &self,
        access_range: RangeInclusive<Address>,
    ) -> impl Iterator<Item = (RangeInclusive<Address>, ComponentId)> {
        self.read.overlapping(access_range.clone())
    }

    #[inline]
    pub fn iter_write(
        &self,
        access_range: RangeInclusive<Address>,
    ) -> impl Iterator<Item = (RangeInclusive<Address>, ComponentId)> {
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

    fn update_members(
        &self,
        commands: impl IntoIterator<Item = MemoryRemappingCommands>,
        registry: &ComponentRegistry,
    ) {
        let invalid_ranges = (0..self.width).complement();
        let mut members = self.members.write().unwrap();

        for command in commands {
            match command {
                MemoryRemappingCommands::RemoveRead { range } => {
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
                MemoryRemappingCommands::RemoveWrite { range } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range,
                            self.width - 1
                        );
                    }

                    tracing::debug!("Removing memory range {:#04x?} from address space", range,);

                    members.write.remove(range.clone());
                }
                MemoryRemappingCommands::MapReadComponent { range, path } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?} (inserted by {})",
                            range,
                            self.width - 1,
                            path
                        );
                    }

                    tracing::debug!(
                        "Mapping read memory to address range {:#04x?} for {}",
                        range,
                        path
                    );

                    let id = registry.get_id(&path).unwrap();
                    members.read.insert(range.clone(), id);
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

                    members.read.insert(range.clone(), id);
                    members.write.insert(range.clone(), id);
                }
            }
        }

        members.read.commit();
        members.write.commit();
    }
}

#[derive(Debug)]
pub enum MemoryRemappingCommands {
    MapReadComponent {
        range: RangeInclusive<Address>,
        path: ComponentPath,
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
