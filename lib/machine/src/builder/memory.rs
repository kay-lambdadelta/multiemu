use crate::builder::ComponentBuilder;
use crate::component::Component;
use crate::memory::AddressSpaceId;
use crate::memory::callbacks::Memory;
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;

#[derive(Default)]
pub struct MemoryMetadata {
    pub memories: Vec<(RangeInclusiveMap<usize, AddressSpaceId>, Box<dyn Memory>)>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    /// Insert a callback into the memory translation table for reading
    pub fn insert_memory(
        mut self,
        assigned_addresses: impl IntoIterator<Item = (RangeInclusive<usize>, AddressSpaceId)>,
        callback: impl Memory,
    ) -> Self {
        let as_memory = self.component_metadata.memory.get_or_insert_default();

        as_memory
            .memories
            .push((assigned_addresses.into_iter().collect(), Box::new(callback)));

        self
    }
}
