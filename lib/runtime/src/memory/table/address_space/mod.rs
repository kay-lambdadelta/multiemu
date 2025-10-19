use crate::{
    component::{ComponentPath, ErasedComponentHandle},
    machine::registry::ComponentRegistry,
    memory::Address,
};
use bitvec::{field::BitField, order::Lsb0};
use multiemu_range::RangeIntersection;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use std::{
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    sync::{
        Arc, Mutex, RwLock, RwLockReadGuard,
        atomic::{AtomicBool, Ordering},
    },
    vec::Vec,
};

mod commit;
mod visit_overlapping;

const PAGE_SIZE: Address = 0x1000;

#[derive(Debug, Clone)]
struct TableEntry {
    /// Full, uncropped relevant range
    pub start: Address,
    pub end: Address,
    /// Mirror offset
    pub mirror_start: Option<Address>,
    /// Handle to component
    pub component: ErasedComponentHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingEntry {
    Component(ComponentPath),
    Mirror {
        source_base: Address,
        destination_base: Address,
    },
}

#[derive(Debug)]
pub struct MemoryMappingTable {
    master: RangeInclusiveMap<Address, MappingEntry>,
    table: Vec<Vec<TableEntry>>,
    registry: Arc<ComponentRegistry>,
}

impl MemoryMappingTable {
    pub fn new(address_space_width: u8, registry: Arc<ComponentRegistry>) -> Self {
        let addr_space_size = 2usize.pow(address_space_width as u32);
        let total_pages = addr_space_size.div_ceil(PAGE_SIZE);

        Self {
            master: RangeInclusiveMap::new(),
            table: vec![Default::default(); total_pages],
            registry,
        }
    }

    pub fn insert_component(&mut self, source_range: RangeInclusive<Address>, path: ComponentPath) {
        self.master
            .insert(source_range, MappingEntry::Component(path));
    }

    pub fn insert_mirror(
        &mut self,
        source_range: RangeInclusive<Address>,
        destination_range: RangeInclusive<Address>,
    ) {
        self.master.insert(
            source_range.clone(),
            MappingEntry::Mirror {
                source_base: *source_range.start(),
                destination_base: *destination_range.start(),
            },
        );
    }

    pub fn remove(&mut self, range: RangeInclusive<Address>) {
        self.master.remove(range);
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

#[derive(Debug)]
pub struct Members {
    pub read: MemoryMappingTable,
    pub write: MemoryMappingTable,
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
    pub fn new(address_space_width: u8, registry: Arc<ComponentRegistry>) -> Self {
        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load_le();

        Self {
            width_mask,
            address_space_width,
            members: RwLock::new(Members {
                read: MemoryMappingTable::new(address_space_width, registry.clone()),
                write: MemoryMappingTable::new(address_space_width, registry.clone()),
            }),
            queue: Default::default(),
            queue_modified: AtomicBool::new(false),
        }
    }

    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommand>) {
        let mut queue_guard = self.queue.lock().unwrap();

        queue_guard.extend(commands);
        self.queue_modified.store(true, Ordering::Release);
    }

    #[inline]
    pub fn get_members(&self) -> RwLockReadGuard<'_, Members> {
        if !self.queue_modified.load(Ordering::Acquire) {
            self.members.read().unwrap()
        } else {
            self.update_members();
            self.members.read().unwrap()
        }
    }

    #[cold]
    fn update_members(&self) {
        let mut queue_guard = self.queue.lock().unwrap();
        self.queue_modified.store(false, Ordering::Release);

        let max = 2usize.pow(self.address_space_width as u32) - 1;

        let valid_range = 0..=max;
        let mut members = self.members.write().unwrap();

        for command in queue_guard.drain(..) {
            match command {
                MemoryRemappingCommand::Component {
                    range,
                    component,
                    permissions,
                } => {
                    if valid_range.disjoint(&range) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?} (inserted by {})",
                            range, max, component
                        );
                    }

                    if permissions.read {
                        members
                            .read
                            .insert_component(range.clone(), component.clone());
                    }

                    if permissions.write {
                        members.write.insert_component(range, component);
                    }
                }
                MemoryRemappingCommand::Unmap { range, permissions } => {
                    if permissions.read {
                        members.read.remove(range.clone());
                    }

                    if permissions.write {
                        members.write.remove(range.clone());
                    }
                }
                MemoryRemappingCommand::Mirror {
                    source: range,
                    destination: destination_range,
                    permissions,
                } => {
                    if valid_range.disjoint(&range) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range, max
                        );
                    }

                    if permissions.read {
                        members
                            .read
                            .insert_mirror(range.clone(), destination_range.clone());
                    }

                    if permissions.write {
                        members.write.insert_mirror(range, destination_range);
                    }
                }
            }
        }

        members.read.commit();
        members.write.commit();
    }
}

#[derive(Debug)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
}

impl Permissions {
    pub fn all() -> Self {
        Self {
            read: true,
            write: true,
        }
    }
}

#[derive(Debug)]
pub enum MemoryRemappingCommand {
    Component {
        range: RangeInclusive<Address>,
        component: ComponentPath,
        permissions: Permissions,
    },
    Mirror {
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        permissions: Permissions,
    },
    Unmap {
        range: RangeInclusive<Address>,
        permissions: Permissions,
    },
}
