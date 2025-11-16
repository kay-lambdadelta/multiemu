use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    sync::{
        Arc, Mutex, RwLock, RwLockWriteGuard,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    component::{ComponentPath, ErasedComponentHandle},
    machine::registry::ComponentRegistry,
};

mod commit;
mod read;
mod visit_overlapping;
mod write;

use bitvec::{field::BitField, order::Lsb0};
use multiemu_range::RangeIntersection;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use thiserror::Error;

pub type Address = usize;
const PAGE_SIZE: Address = 0x1000;

/// The main structure representing the devices memory address spaces
#[derive(Debug)]
pub struct AddressSpace {
    width_mask: Address,
    address_space_width: u8,
    members: RwLock<Members>,
    /// Queue for if the address space is locked at the moment
    queue: Mutex<Vec<MemoryRemappingCommand>>,
    queue_modified: AtomicBool,
    id: AddressSpaceId,
}

impl AddressSpace {
    pub(crate) fn new(
        registry: Arc<ComponentRegistry>,
        address_space_id: AddressSpaceId,
        address_space_width: u8,
    ) -> Self {
        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load_le();

        Self {
            id: address_space_id,
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

    /// Adds a command to the remap queue
    ///
    /// Note that the queue is not applied till the next memory operation
    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommand>) {
        let mut queue_guard = self.queue.lock().unwrap();

        queue_guard.extend(commands);
        self.queue_modified.store(true, Ordering::Release);
    }

    #[inline]
    fn interact_members(
        &self,
        callback: impl FnOnce(&Members) -> Result<(), MemoryError>,
    ) -> Result<(), MemoryError> {
        if !self.queue_modified.load(Ordering::Acquire) {
            callback(&self.members.read().unwrap())
        } else {
            let members = self.update_members();

            callback(&members)
        }
    }

    pub fn commit(&self) {
        let _ = self.interact_members(|_| Ok(()));
    }

    #[cold]
    fn update_members(&self) -> RwLockWriteGuard<'_, Members> {
        let mut queue_guard = self.queue.lock().unwrap();
        self.queue_modified.store(false, Ordering::Release);

        let max = 2usize.pow(u32::from(self.address_space_width)) - 1;

        let valid_range = 0..=max;
        let mut members = self.members.write().unwrap();

        for command in queue_guard.drain(..) {
            match command {
                MemoryRemappingCommand::Component {
                    range,
                    component,
                    permissions,
                } => {
                    assert!(
                        !valid_range.disjoint(&range),
                        "Range {range:#04x?} is invalid for a address space that ends at {max:04x?} (inserted by {component})"
                    );

                    tracing::debug!(
                        "Mapping component {component} to range {range:#04x?} with permissions {:?}",
                        permissions
                    );

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
                    assert!(
                        !valid_range.disjoint(&range),
                        "Range {range:#04x?} is invalid for a address space that ends at {max:04x?}"
                    );

                    tracing::debug!(
                        "Mapping mirror to range {range:#04x?} with permissions {:?} from range {destination_range:#04x?}",
                        permissions
                    );

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

        members
    }

    pub fn id(&self) -> AddressSpaceId {
        self.id
    }
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
}

impl Permissions {
    /// Instance of [Self] where everything is allowed
    pub fn all() -> Self {
        Self {
            read: true,
            write: true,
        }
    }
}

/// Command for how the memory access table should modify the memory map
#[allow(missing_docs)]
#[derive(Debug)]
pub enum MemoryRemappingCommand {
    /// Add a component to the memory map, or add a map to an existing one
    Component {
        range: RangeInclusive<Address>,
        component: ComponentPath,
        permissions: Permissions,
    },
    /// Add a mirror to the memory map
    Mirror {
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        permissions: Permissions,
    },
    /// Clear a memory range
    Unmap {
        range: RangeInclusive<Address>,
        permissions: Permissions,
    },
}

#[derive(Clone)]
struct TableEntry {
    /// Full, uncropped relevant range
    pub start: Address,
    pub end: Address,
    /// Mirror offset
    pub mirror_start: Option<Address>,
    /// Handle to component
    pub component: ErasedComponentHandle,
}

impl Debug for TableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableEntry")
            .field("start", &self.start)
            .field("end", &self.end)
            .field("mirror_start", &self.mirror_start)
            .finish()
    }
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
        let addr_space_size = 2usize.pow(u32::from(address_space_width));
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a read operation failed
pub enum MemoryErrorType {
    /// Access was denied
    Denied,
    /// Nothing is mapped there
    OutOfBus,
    /// It would be impossible to view this memory without a state change
    Impossible,
}

#[derive(Error, Debug)]
#[error("Memory operation failed: {0:#x?}")]
/// Wrapper around the error type in order to specify ranges
pub struct MemoryError(pub RangeInclusiveMap<Address, MemoryErrorType>);
