use super::Address;
use crate::component::ComponentRegistry;
use address_space::AddressSpace;
use nohash::BuildNoHashHasher;
use std::{collections::HashMap, ops::RangeInclusive, sync::Arc};

mod address_space;
mod read;
mod write;

pub use address_space::{AddressSpaceId, MemoryRemappingCommands, MemoryType};
pub use read::*;
pub use write::*;

#[allow(clippy::type_complexity)]
/// Error type from componenents
#[derive(Debug)]
pub struct MemoryOperationError<R> {
    /// Records the memory translation table should handle
    pub records: Vec<(RangeInclusive<Address>, R)>,
}

impl<R> FromIterator<(RangeInclusive<Address>, R)> for MemoryOperationError<R> {
    fn from_iter<T: IntoIterator<Item = (RangeInclusive<Address>, R)>>(iter: T) -> Self {
        Self {
            records: iter.into_iter().collect(),
        }
    }
}

#[derive(Debug)]
/// The main structure representing the devices memory address spaces
pub struct MemoryAccessTable {
    address_spaces: HashMap<AddressSpaceId, AddressSpace, BuildNoHashHasher<u16>>,
    current_address_space: u16,
    registry: Arc<ComponentRegistry>,
}

impl MemoryAccessTable {
    pub(crate) fn new(component_store: Arc<ComponentRegistry>) -> Self {
        Self {
            address_spaces: Default::default(),
            current_address_space: 0,
            registry: component_store,
        }
    }

    pub(crate) fn insert_address_space(&mut self, address_space_width: u8) -> AddressSpaceId {
        let id = AddressSpaceId::new(self.current_address_space);

        self.current_address_space = self
            .current_address_space
            .checked_add(1)
            .expect("Too many address spaces");

        self.address_spaces.insert(
            id,
            AddressSpace::new(self.registry.clone(), address_space_width),
        );

        id
    }

    /// Iter over present spaces
    pub fn address_spaces(&self) -> impl Iterator<Item = AddressSpaceId> {
        self.address_spaces.keys().copied()
    }

    /// Remap memory in a specific address space, clearing previous mappings
    pub fn remap(
        &self,
        address_space: AddressSpaceId,
        commands: impl IntoIterator<Item = MemoryRemappingCommands>,
    ) {
        let address_space = &self.address_spaces[&address_space];
        address_space.remap(commands);
    }
}

#[derive(Debug)]
struct QueueEntry {
    address: Address,
    address_space: AddressSpaceId,
    buffer_subrange: RangeInclusive<Address>,
}
