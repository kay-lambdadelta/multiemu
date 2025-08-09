use super::Address;
use crate::component::ComponentRegistry;
use address_space::AddressSpace;
use bitvec::{field::BitField, order::Lsb0};
use nohash::BuildNoHashHasher;
use rangemap::RangeInclusiveMap;
use std::{
    collections::HashMap,
    ops::RangeInclusive,
    sync::{
        Arc,
        atomic::{AtomicU16, Ordering},
    },
};

mod address_space;
mod read;
mod write;

pub use address_space::{AddressSpaceHandle, MemoryRemappingCommands, MemoryType};
pub use read::*;
pub use write::*;

#[allow(clippy::type_complexity)]
/// Error type from componenents
pub struct MemoryOperationError<R> {
    /// Records the memory translation table should handle
    pub records: RangeInclusiveMap<Address, R>,
    /// Allows remapping when its safe. The semantics of when this occurs is unspecified except that the caller that triggered this will not return until the remap(s) occurs.
    pub remapping_commands: Vec<(AddressSpaceHandle, Vec<MemoryRemappingCommands>)>,
}

impl<R> From<RangeInclusiveMap<Address, R>> for MemoryOperationError<R> {
    fn from(records: RangeInclusiveMap<Address, R>) -> Self {
        Self {
            records,
            remapping_commands: Default::default(),
        }
    }
}
#[derive(Debug)]
/// The main structure representing the devices memory address spaces
pub struct MemoryAccessTable {
    address_spaces: HashMap<AddressSpaceHandle, AddressSpace, BuildNoHashHasher<u16>>,
    current_address_space: AtomicU16,
    component_store: Arc<ComponentRegistry>,
}

impl MemoryAccessTable {
    pub(crate) fn new(component_store: Arc<ComponentRegistry>) -> Self {
        Self {
            address_spaces: Default::default(),
            current_address_space: AtomicU16::new(1),
            component_store,
        }
    }

    pub(crate) fn insert_address_space(&mut self, address_space_width: u8) -> AddressSpaceHandle {
        let id =
            AddressSpaceHandle::new(self.current_address_space.fetch_add(1, Ordering::Relaxed))
                .unwrap();

        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load();

        self.address_spaces
            .insert(id, AddressSpace::new(width_mask));

        id
    }

    /// Iter over present spaces
    pub fn address_spaces(&self) -> impl Iterator<Item = AddressSpaceHandle> {
        self.address_spaces.keys().copied()
    }

    /// Remap memory in a specific address space, clearing previous mappings
    pub fn remap(
        &self,
        address_space: AddressSpaceHandle,
        commands: impl IntoIterator<Item = MemoryRemappingCommands>,
    ) {
        let address_space = self.address_spaces.get(&address_space).unwrap();
        address_space.remap(commands);
    }
}

#[derive(Debug)]
struct QueueEntry {
    address: Address,
    address_space: AddressSpaceHandle,
    buffer_subrange: RangeInclusive<Address>,
}
