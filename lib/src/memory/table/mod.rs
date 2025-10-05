use crate::machine::registry::ComponentRegistry;

use super::Address;
use address_space::AddressSpace;
use nohash::BuildNoHashHasher;
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::RangeInclusive,
    sync::{Arc, OnceLock},
};

mod address_space;
mod read;
mod write;

pub use address_space::{AddressSpaceId, MemoryRemappingCommands};
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

#[derive(Default)]
/// The main structure representing the devices memory address spaces
pub struct MemoryAccessTable {
    address_spaces: HashMap<AddressSpaceId, AddressSpace, BuildNoHashHasher<u16>>,
    current_address_space: u16,
    registry: OnceLock<Arc<ComponentRegistry>>,
}

impl Debug for MemoryAccessTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryAccessTable")
            .field("address_spaces", &self.address_spaces)
            .finish()
    }
}

impl MemoryAccessTable {
    pub(crate) fn set_registry(&self, registry: Arc<ComponentRegistry>) {
        self.registry.set(registry).unwrap();
    }

    pub(crate) fn insert_address_space(&mut self, address_space_width: u8) -> AddressSpaceId {
        let id = AddressSpaceId::new(self.current_address_space);

        self.current_address_space = self
            .current_address_space
            .checked_add(1)
            .expect("Too many address spaces");

        self.address_spaces
            .insert(id, AddressSpace::new(address_space_width));

        id
    }

    /// Iter over present spaces
    pub fn address_spaces(&self) -> impl Iterator<Item = AddressSpaceId> {
        self.address_spaces.keys().copied()
    }

    pub fn force_remap_commit(&self, address_space: AddressSpaceId) {
        drop(self.address_spaces[&address_space].get_members(self.registry.get().unwrap()));
    }

    /// Adds a command to the remap queue
    ///
    /// Note that the queue is not applied till the next memory operation
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

