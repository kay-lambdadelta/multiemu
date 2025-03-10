use crate::builder::ComponentBuilder;
use crate::component::Component;
use crate::memory::AddressSpaceId;
use crate::memory::callbacks::{PreviewMemory, ReadMemory, WriteMemory};
use rangemap::RangeInclusiveSet;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;

#[derive(Default)]
pub struct MemoryMetadata {
    pub read: HashMap<AddressSpaceId, (RangeInclusiveSet<usize>, Arc<dyn ReadMemory>)>,
    pub write: HashMap<AddressSpaceId, (RangeInclusiveSet<usize>, Arc<dyn WriteMemory>)>,
    pub preview: HashMap<AddressSpaceId, (RangeInclusiveSet<usize>, Arc<dyn PreviewMemory>)>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    /// Insert a callback into the memory translation table for reading
    pub fn insert_read_memory(
        mut self,
        address_space: AddressSpaceId,
        assigned_ranges: impl IntoIterator<Item = RangeInclusive<usize>>,
        callback: Arc<dyn ReadMemory>,
    ) -> Self {
        let as_memory = self.component_metadata.memory.get_or_insert_default();

        as_memory.read.insert(
            address_space,
            (assigned_ranges.into_iter().collect(), callback),
        );

        self
    }

    /// Insert a callback into the memory translation table for writing
    pub fn insert_write_memory(
        mut self,
        address_space: AddressSpaceId,
        assigned_ranges: impl IntoIterator<Item = RangeInclusive<usize>>,
        callback: Arc<dyn WriteMemory>,
    ) -> Self {
        let as_memory = self.component_metadata.memory.get_or_insert_default();

        as_memory.write.insert(
            address_space,
            (assigned_ranges.into_iter().collect(), callback),
        );

        self
    }

    /// Insert a callback into the memory translation table for preview
    pub fn insert_preview_memory(
        mut self,
        address_space: AddressSpaceId,
        assigned_ranges: impl IntoIterator<Item = RangeInclusive<usize>>,
        callback: Arc<dyn PreviewMemory>,
    ) -> Self {
        let as_memory = self.component_metadata.memory.get_or_insert_default();

        as_memory.preview.insert(
            address_space,
            (assigned_ranges.into_iter().collect(), callback),
        );

        self
    }
}
