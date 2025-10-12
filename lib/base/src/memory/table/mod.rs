use crate::machine::registry::ComponentRegistry;
use address_space::AddressSpace;
use std::{
    fmt::Debug,
    sync::{Arc, OnceLock},
};

mod address_space;
mod read;
mod write;

pub use address_space::{AddressSpaceId, MappingPermissions, MemoryRemappingCommand};
pub use read::*;
pub use write::*;

#[derive(Default)]
/// The main structure representing the devices memory address spaces
pub struct MemoryAccessTable {
    address_spaces: Vec<AddressSpace>,
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
            .push(AddressSpace::new(address_space_width));

        id
    }

    /// Iter over present spaces
    pub fn address_spaces(&self) -> impl Iterator<Item = AddressSpaceId> {
        (0..self.address_spaces.len()).map(|space| AddressSpaceId(space as u16))
    }

    pub fn force_remap_commit(&self, address_space: AddressSpaceId) {
        drop(
            self.address_spaces[address_space.0 as usize].get_members(self.registry.get().unwrap()),
        );
    }

    /// Adds a command to the remap queue
    ///
    /// Note that the queue is not applied till the next memory operation
    pub fn remap(
        &self,
        address_space: AddressSpaceId,
        commands: impl IntoIterator<Item = MemoryRemappingCommand>,
    ) {
        let address_space = &self.address_spaces[address_space.0 as usize];
        address_space.remap(commands);
    }
}
