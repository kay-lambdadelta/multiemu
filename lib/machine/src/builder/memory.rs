use crate::{
    builder::ComponentBuilder,
    component::Component,
    memory::{
        AddressSpaceHandle,
        callbacks::{ReadMemory, WriteMemory},
    },
};
use std::{ops::RangeInclusive, sync::Arc};

impl<C: Component> ComponentBuilder<'_, C> {
    /// Insert a callback into the memory translation table for reading
    pub fn insert_read_memory<R: ReadMemory>(
        self,
        callback: impl Into<Arc<R>>,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> Self {
        self.machine_builder
            .essentials
            .memory_translation_table
            .insert_read_memory(callback.into(), assigned_addresses);
        self
    }

    pub fn insert_write_memory<W: WriteMemory>(
        self,
        callback: impl Into<Arc<W>>,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> Self {
        self.machine_builder
            .essentials
            .memory_translation_table
            .insert_write_memory(callback.into(), assigned_addresses);

        self
    }

    pub fn insert_rw_memory<RW: ReadMemory + WriteMemory>(
        self,
        callback: impl Into<Arc<RW>>,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> Self {
        let callback = callback.into();
        let assigned_addresses: Vec<_> = assigned_addresses.into_iter().collect();

        self.machine_builder
            .essentials
            .memory_translation_table
            .insert_read_memory(callback.clone(), assigned_addresses.clone());
        self.machine_builder
            .essentials
            .memory_translation_table
            .insert_write_memory(callback, assigned_addresses);

        self
    }
}
