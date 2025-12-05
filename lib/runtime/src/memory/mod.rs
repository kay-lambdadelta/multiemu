use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    sync::Arc,
};

use arc_swap::{ArcSwap, Cache};
use bitvec::{field::BitField, order::Lsb0};
use bytes::Bytes;
pub use commit::{MapTarget, MemoryRemappingCommand, Permissions};
use multiemu_range::RangeIntersection;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use thiserror::Error;

use crate::{
    component::{ComponentHandle, ComponentPath, ResourcePath},
    machine::registry::ComponentRegistry,
};

mod commit;
mod overlapping;
mod read;
mod write;

pub type Address = usize;
const PAGE_SIZE: Address = 0x1000;

/// The main structure representing the devices memory address spaces
#[derive(Debug)]
pub struct AddressSpace {
    width_mask: Address,
    address_space_width: u8,
    id: AddressSpaceId,
    members: Arc<ArcSwap<Members>>,
    resources: scc::HashMap<ResourcePath, Bytes>,
}

impl AddressSpace {
    pub(crate) fn new(address_space_id: AddressSpaceId, address_space_width: u8) -> Self {
        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load_le();

        Self {
            id: address_space_id,
            width_mask,
            address_space_width,
            members: Arc::new(ArcSwap::new(Arc::new(Members {
                read: MemoryMappingTable::new(address_space_width),
                write: MemoryMappingTable::new(address_space_width),
            }))),
            resources: scc::HashMap::default(),
        }
    }

    pub fn cache(&self) -> AddressSpaceCache {
        AddressSpaceCache {
            members: Cache::new(self.members.clone()),
        }
    }

    pub(crate) fn remap(
        &self,
        commands: impl IntoIterator<Item = MemoryRemappingCommand>,
        registry: &ComponentRegistry,
    ) {
        let max = 2usize.pow(u32::from(self.address_space_width)) - 1;
        let valid_range = 0..=max;
        let commands: Vec<_> = commands.into_iter().collect();

        self.members.rcu(|members| {
            let mut members = Members::clone(members);

            for command in commands.clone() {
                match command {
                    MemoryRemappingCommand::Map {
                        range,
                        target,
                        permissions,
                    } => {
                        assert!(
                            !valid_range.disjoint(&range),
                            "Range {range:#04x?} is invalid for a address space that ends at \
                             {max:04x?}"
                        );

                        match target {
                            MapTarget::Component(component_path) => {
                                if permissions.read {
                                    members.read.master.insert(
                                        range.clone(),
                                        MappingEntry::Component(component_path.clone()),
                                    );
                                }

                                if permissions.write {
                                    members.write.master.insert(
                                        range.clone(),
                                        MappingEntry::Component(component_path),
                                    );
                                }
                            }
                            MapTarget::Memory(resource_path) => {
                                assert!(self.resources.contains_sync(&resource_path));

                                if permissions.read {
                                    members.read.master.insert(
                                        range.clone(),
                                        MappingEntry::Memory(resource_path.clone()),
                                    );
                                }

                                if permissions.write {
                                    members
                                        .write
                                        .master
                                        .insert(range.clone(), MappingEntry::Memory(resource_path));
                                }
                            }
                            MapTarget::Mirror { destination } => {
                                assert!(
                                    !valid_range.disjoint(&destination),
                                    "Range {destination:#04x?} is invalid for a address space \
                                     that ends at {max:04x?}"
                                );

                                if permissions.read {
                                    members.read.master.insert(
                                        range.clone(),
                                        MappingEntry::Mirror {
                                            source_base: *range.start(),
                                            destination_base: *destination.start(),
                                        },
                                    );
                                }

                                if permissions.write {
                                    members.write.master.insert(
                                        range.clone(),
                                        MappingEntry::Mirror {
                                            source_base: *range.start(),
                                            destination_base: *destination.start(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                    MemoryRemappingCommand::Unmap { range, permissions } => {
                        if permissions.read {
                            members.read.master.remove(range.clone());
                        }

                        if permissions.write {
                            members.write.master.remove(range.clone());
                        }
                    }
                    MemoryRemappingCommand::Register { id, buffer } => {
                        self.resources.insert_sync(id, buffer).unwrap();
                    }
                }
            }

            members.read.commit(registry, &self.resources);
            members.write.commit(registry, &self.resources);

            members
        });
    }

    pub fn id(&self) -> AddressSpaceId {
        self.id
    }
}

#[derive(Clone)]
pub enum ComputedTablePageTarget {
    Component {
        mirror_start: Option<Address>,
        component: ComponentHandle,
    },
    Memory(Bytes),
}

#[derive(Clone)]
struct ComputedTablePage {
    /// Full, uncropped relevant range
    pub range: RangeInclusive<Address>,
    pub target: ComputedTablePageTarget,
}

impl Debug for ComputedTablePage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableEntry")
            .field("range", &self.range)
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
    Memory(ResourcePath),
}

#[derive(Debug, Clone)]
pub struct MemoryMappingTable {
    master: RangeInclusiveMap<Address, MappingEntry>,
    computed_table: Vec<Vec<ComputedTablePage>>,
}

impl MemoryMappingTable {
    pub fn new(address_space_width: u8) -> Self {
        let addr_space_size = 2usize.pow(u32::from(address_space_width));
        let total_pages = addr_space_size.div_ceil(PAGE_SIZE);

        Self {
            master: RangeInclusiveMap::new(),
            computed_table: vec![Default::default(); total_pages],
        }
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

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct AddressSpaceCache {
    members: Cache<Arc<ArcSwap<Members>>, Arc<Members>>,
}
