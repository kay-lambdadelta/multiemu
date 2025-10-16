use crate::{
    component::{Component, ComponentPath},
    machine::registry::ComponentRegistry,
    memory::Address,
};
use bitvec::{field::BitField, order::Lsb0};
use itertools::Itertools;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use rayon::{iter::IntoParallelIterator, prelude::ParallelIterator};
use std::{
    error::Error,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut, RangeInclusive},
    sync::{
        Arc, Mutex, RwLock, RwLockReadGuard,
        atomic::{AtomicBool, Ordering},
    },
    vec::Vec,
};

const PAGE_SIZE: Address = 0x1000;

#[derive(Debug)]
struct MixedTableEntry {
    /// Full, uncropped relevant range
    pub start: Address,
    pub end: Address,
    pub component: Arc<RwLock<dyn Component>>,
}

#[derive(Debug)]
enum Page {
    Empty,
    Single {
        /// Full, uncropped relevant range
        start: Address,
        end: Address,
        component: Arc<RwLock<dyn Component>>,
    },
    Mixed {
        components: Vec<MixedTableEntry>,
    },
}

#[derive(Debug, Default)]
pub struct MemoryMappingTable {
    master: RangeInclusiveMap<Address, ComponentPath>,
    table: Vec<Page>,
}

impl MemoryMappingTable {
    #[inline(always)]
    pub fn visit_overlapping<E>(
        &self,
        access_range: RangeInclusive<Address>,
        mut visitor: impl FnMut(RangeInclusive<Address>, &RwLock<dyn Component>) -> Result<(), E>,
    ) -> Result<(), E> {
        let start = *access_range.start();
        let end = *access_range.end();

        let start_page = start / PAGE_SIZE;
        let end_page = end / PAGE_SIZE;

        for page_index in start_page..=end_page {
            let page = &self.table[page_index];

            match page {
                Page::Empty => {}
                Page::Single {
                    start,
                    end,
                    component,
                } => {
                    let range = *start..=*end;

                    visitor(range.clone(), component)?;

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
                        .map(|entry| (entry.start..=entry.end, &entry.component))
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

    pub fn insert(&mut self, range: RangeInclusive<Address>, component: ComponentPath) {
        self.master.insert(range, component);
    }

    pub fn remove(&mut self, range: RangeInclusive<Address>) {
        self.master.remove(range);
    }

    pub fn commit(&mut self, address_space_width: u8, registry: &ComponentRegistry) {
        self.table.clear();

        let max = 2usize.pow(address_space_width as u32) - 1;
        let total_pages = (max + 1) / PAGE_SIZE;

        // Process all pages in parallel
        let new_table: Vec<Page> = (0..total_pages)
            .into_par_iter()
            .map(|page_index| {
                let base = page_index * PAGE_SIZE;
                let end = base + PAGE_SIZE - 1;
                let page_range = base..=end;

                let mut entries: Vec<_> = self
                    .master
                    .overlapping(page_range.clone())
                    .map(|(range, component)| (range.clone(), component))
                    .collect();

                match entries.len() {
                    0 => Page::Empty,
                    1 => {
                        let (range, component) = entries.remove(0);
                        let test_range: RangeInclusive<Address> =
                            range.clone().intersection(page_range.clone()).into();

                        let component = registry.get_direct(component).unwrap();

                        if test_range == page_range {
                            Page::Single {
                                component,
                                start: *range.start(),
                                end: *range.end(),
                            }
                        } else {
                            Page::Mixed {
                                components: vec![MixedTableEntry {
                                    component,
                                    start: *range.start(),
                                    end: *range.end(),
                                }],
                            }
                        }
                    }
                    _ => {
                        let inner_table: Vec<_> = entries
                            .drain(..)
                            .map(|(range, component)| {
                                let component = registry.get_direct(component).unwrap();

                                MixedTableEntry {
                                    component,
                                    start: *range.start(),
                                    end: *range.end(),
                                }
                            })
                            .sorted_by_key(|entry| entry.start)
                            .collect();

                        Page::Mixed {
                            components: inner_table,
                        }
                    }
                }
            })
            .collect();

        self.table = new_table;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
/// Identifier for a address space
pub struct AddressSpaceId(pub(crate) u16);

impl AddressSpaceId {
    pub(crate) fn new(id: u16) -> Self {
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
    #[inline(always)]
    pub fn visit_read<E: Error>(
        &self,
        access_range: RangeInclusive<Address>,
        mut visitor: impl FnMut(RangeInclusive<Address>, &dyn Component) -> Result<(), E>,
    ) -> Result<(), E> {
        self.read
            .visit_overlapping(access_range.clone(), |range, component| {
                visitor(range, component.read().unwrap().deref())
            })
    }

    #[inline(always)]
    pub fn visit_write<E: Error>(
        &self,
        access_range: RangeInclusive<Address>,
        mut visitor: impl FnMut(RangeInclusive<Address>, &mut dyn Component) -> Result<(), E>,
    ) -> Result<(), E> {
        self.write
            .visit_overlapping(access_range.clone(), |range, component| {
                visitor(range, component.write().unwrap().deref_mut())
            })
    }
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    address_space_width: u8,
    members: RwLock<Members>,
    /// Queue for if the address space is locked at the moment
    queue: Mutex<Vec<MemoryRemappingCommand>>,
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

    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommand>) {
        let mut queue_guard = self.queue.lock().unwrap();

        queue_guard.extend(commands);
        self.queue_modified.store(true, Ordering::Release);
    }

    #[inline(always)]
    pub fn get_members(&self, registry: &ComponentRegistry) -> RwLockReadGuard<'_, Members> {
        if !self.queue_modified.load(Ordering::Acquire) {
            self.members.read().unwrap()
        } else {
            self.update_members(registry);
            self.members.read().unwrap()
        }
    }

    #[cold]
    fn update_members(&self, registry: &ComponentRegistry) {
        let mut queue_guard = self.queue.lock().unwrap();
        self.queue_modified.store(false, Ordering::Release);

        let max = 2usize.pow(self.address_space_width as u32) - 1;

        let invalid_ranges = (0..=max).complement();
        let mut members = self.members.write().unwrap();

        for command in queue_guard.drain(..) {
            match command {
                MemoryRemappingCommand::Remap {
                    range,
                    permissions,
                    component,
                } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?} (inserted by {})",
                            range, max, component
                        );
                    }

                    if permissions.contains(&MappingPermissions::Read) {
                        members.read.insert(range.clone(), component.clone());
                    }

                    if permissions.contains(&MappingPermissions::Write) {
                        members.write.insert(range, component);
                    }
                }
                MemoryRemappingCommand::Unmap { range, permissions } => {
                    if permissions.contains(&MappingPermissions::Read) {
                        members.read.remove(range.clone());
                    }

                    if permissions.contains(&MappingPermissions::Write) {
                        members.write.remove(range.clone());
                    }
                }
            }
        }

        members.read.commit(self.address_space_width, registry);
        members.write.commit(self.address_space_width, registry);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MappingPermissions {
    Read,
    Write,
}

#[derive(Debug)]
pub enum MemoryRemappingCommand {
    Remap {
        range: RangeInclusive<Address>,
        permissions: Vec<MappingPermissions>,
        component: ComponentPath,
    },
    Unmap {
        range: RangeInclusive<Address>,
        permissions: Vec<MappingPermissions>,
    },
}
