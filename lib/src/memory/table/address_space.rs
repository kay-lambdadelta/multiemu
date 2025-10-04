use crate::{
    component::{ComponentId, ComponentPath},
    machine::registry::ComponentRegistry,
    memory::Address,
};
use bitvec::{field::BitField, order::Lsb0};
use itertools::Itertools;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::{
    error::Error,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    sync::{
        Mutex, RwLock, RwLockReadGuard,
        atomic::{AtomicBool, Ordering},
    },
    vec::Vec,
};

const PAGE_SIZE: Address = 0x1000;

#[derive(Debug)]
struct MixedTableEntry {
    pub component: ComponentId,
    /// Full, uncropped relevant range
    pub start: Address,
    pub end: Address,
}

#[derive(Debug)]
enum Page {
    Empty,
    Single {
        component: ComponentId,
        /// Full, uncropped relevant range
        start: Address,
        end: Address,
    },
    Mixed {
        components: Vec<MixedTableEntry>,
    },
}

#[derive(Debug, Default)]
pub struct MemoryMappingTable {
    master: RangeInclusiveMap<Address, ComponentId>,
    table: Vec<Page>,
}

impl MemoryMappingTable {
    #[inline]
    pub fn visit_overlapping<E>(
        &self,
        access_range: RangeInclusive<Address>,
        mut visitor: impl FnMut(RangeInclusive<Address>, ComponentId) -> Result<(), E>,
    ) -> Result<(), E> {
        let start = *access_range.start();
        let end = *access_range.end();

        let start_page = start / PAGE_SIZE;
        let end_page = end / PAGE_SIZE;

        for page_index in start_page..=end_page {
            let page = &self.table[page_index];

            match page {
                Page::Empty => {
                    continue;
                }
                Page::Single {
                    component,
                    start,
                    end,
                } => {
                    let range = *start..=*end;

                    visitor(range.clone(), *component)?;

                    // If this range completely contains our accessing range we can exit early without more searching

                    let test_range: RangeInclusive<Address> =
                        range.intersection(access_range.clone()).into();

                    if test_range == access_range {
                        return Ok(());
                    }
                }
                // Do a binary search
                Page::Mixed { components } => {
                    let index = components
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

                    let left = components[..index]
                        .iter()
                        .rev()
                        .take_while(move |entry| entry.end >= start);

                    let right = components[index..]
                        .iter()
                        .take_while(move |entry| entry.start <= end);

                    for (range, component) in left
                        .chain(right)
                        .map(|entry| (entry.start..=entry.end, entry.component))
                    {
                        visitor(range.clone(), component)?;

                        let test_range: RangeInclusive<Address> =
                            range.intersection(access_range.clone()).into();

                        if test_range == access_range {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn insert(&mut self, range: RangeInclusive<Address>, component: ComponentId) {
        self.master.insert(range, component);
    }

    pub fn remove(&mut self, range: RangeInclusive<Address>) {
        self.master.remove(range);
    }

    pub fn commit(&mut self, address_space_width: u8) {
        self.table.clear();

        let max = 2usize.pow(address_space_width as u32) - 1;
        let mut entries = Vec::default();

        for base in (0..=max).step_by(PAGE_SIZE) {
            let end = base + PAGE_SIZE - 1;
            let page_range = base..=end;

            entries.extend(
                self.master
                    .overlapping(page_range.clone())
                    .map(|(range, component)| (range.clone(), *component)),
            );

            match entries.len() {
                0 => {
                    self.table.push(Page::Empty);
                }
                1 => {
                    let (range, component) = entries.remove(0);

                    let test_range: RangeInclusive<Address> =
                        range.clone().intersection(page_range.clone()).into();

                    if test_range == page_range {
                        self.table.push(Page::Single {
                            component,
                            start: *range.start(),
                            end: *range.end(),
                        });
                    } else {
                        self.table.push(Page::Mixed {
                            components: vec![MixedTableEntry {
                                component,
                                start: *range.start(),
                                end: *range.end(),
                            }],
                        });
                    }
                }
                _ => {
                    let inner_table: Vec<_> = entries
                        .drain(..)
                        .map(|(range, component)| MixedTableEntry {
                            component,
                            start: *range.start(),
                            end: *range.end(),
                        })
                        .sorted_by_key(|entry| entry.start)
                        .collect();

                    self.table.push(Page::Mixed {
                        components: inner_table,
                    });
                }
            }
        }
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
    pub fn visit_read<E: Error>(
        &self,
        access_range: RangeInclusive<Address>,
        visitor: impl FnMut(RangeInclusive<Address>, ComponentId) -> Result<(), E>,
    ) -> Result<(), E> {
        self.read.visit_overlapping(access_range.clone(), visitor)
    }

    #[inline]
    pub fn visit_write<E: Error>(
        &self,
        access_range: RangeInclusive<Address>,
        visitor: impl FnMut(RangeInclusive<Address>, ComponentId) -> Result<(), E>,
    ) -> Result<(), E> {
        self.write.visit_overlapping(access_range.clone(), visitor)
    }
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    address_space_width: u8,
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

        Self {
            width_mask,
            address_space_width,
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
        let max = 2usize.pow(self.address_space_width as u32) - 1;

        let invalid_ranges = (0..=max).complement();
        let mut members = self.members.write().unwrap();

        for command in commands {
            match command {
                MemoryRemappingCommands::RemoveRead { range } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range, max
                        );
                    }

                    tracing::debug!("Removing memory range {:#04x?} from address space", range,);

                    members.read.remove(range.clone());
                }
                MemoryRemappingCommands::RemoveWrite { range } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range, max
                        );
                    }

                    tracing::debug!("Removing memory range {:#04x?} from address space", range,);

                    members.write.remove(range.clone());
                }
                MemoryRemappingCommands::MapReadComponent { range, path } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?} (inserted by {})",
                            range, max, path
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
                            range, max, path
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
                            range, max, path
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

        members.read.commit(self.address_space_width);
        members.write.commit(self.address_space_width);
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn single_mapping_basic_visit() {
        let mut table = MemoryMappingTable::default();
        let range = 0x1000..=0x1fff;
        let component = ComponentId::new(0);

        table.insert(range.clone(), component);
        table.commit(16);

        let mut visited = Vec::new();
        table
            .visit_overlapping::<()>(0x1fff..=0x1fff, |range, component| {
                visited.push((range, component));

                Ok(())
            })
            .unwrap();

        assert_eq!(visited.len(), 1);
        assert_eq!(visited[0], (range, component));
    }

    #[test]
    fn empty_table_no_visits() {
        let mut table = MemoryMappingTable::default();
        table.commit(16);

        let mut visited = Vec::new();
        table
            .visit_overlapping::<()>(0x1000..=0x1fff, |range, component| {
                visited.push((range, component));
                Ok(())
            })
            .unwrap();

        assert!(visited.is_empty());
    }

    #[test]
    fn mapping_spans_multiple_pages() {
        let mut table = MemoryMappingTable::default();
        let span = 0x0000..=0x2fff;
        let component = ComponentId::new(0);

        table.insert(span.clone(), component);
        table.commit(16);

        for access in [
            0x0000..=0x0000,
            0x0fff..=0x0fff,
            0x1000..=0x1000,
            0x2ffe..=0x2fff,
        ] {
            let mut visited = Vec::new();
            table
                .visit_overlapping::<()>(access, |range, component| {
                    visited.push((range, component));
                    Ok(())
                })
                .unwrap();

            // should only be reported once even though it spans multiple pages
            assert_eq!(visited.len(), 1);
            assert_eq!(visited[0], (span.clone(), component));
        }
    }

    #[test]
    fn remove_mapping_prevents_visit() {
        let mut table = MemoryMappingTable::default();
        let range = 0x1000..=0x1fff;
        let component = ComponentId::new(0);

        table.insert(range.clone(), component);
        table.commit(16);

        // Confirm it's there
        let mut count = 0;
        table
            .visit_overlapping::<()>(range.clone(), |_r, _c| {
                count += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(count, 1);

        // Remove and commit
        table.remove(range.clone());
        table.commit(16);

        count = 0;
        table
            .visit_overlapping::<()>(range.clone(), |_r, _c| {
                count += 1;

                Ok(())
            })
            .unwrap();

        assert_eq!(count, 0);
    }
}
