use crate::{
    builder::ComponentBuilder,
    component::Component,
    display::backend::RenderApi,
    memory::{
        AddressSpaceHandle,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::MemoryHandle,
    },
};
use std::ops::RangeInclusive;

impl<R: RenderApi, C: Component> ComponentBuilder<'_, R, C> {
    /// Insert a callback into the memory translation table for reading
    pub fn insert_read_memory<M: ReadMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_read_memory(callback, assigned_addresses);

        (self, memory_handle)
    }

    pub fn insert_write_memory<M: WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_write_memory(callback, assigned_addresses);

        (self, memory_handle)
    }

    pub fn insert_memory<M: ReadMemory + WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_memory(callback, assigned_addresses);

        (self, memory_handle)
    }
}
