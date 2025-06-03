use crate::{
    builder::ComponentBuilder,
    component::Component,
    display::backend::RenderApi,
    memory::{
        Address,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{MemoryHandle, address_space::AddressSpaceHandle},
    },
};
use rangemap::RangeInclusiveSet;
use std::{collections::HashMap, ops::RangeInclusive};

impl<R: RenderApi, C: Component> ComponentBuilder<'_, R, C> {
    /// Insert a callback into the memory translation table for reading
    pub fn insert_read_memory<M: ReadMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_read_memory(callback);

        // Merge all the addresses together so we can remap them without erasing previous ones
        // TODO: Explore remapping without erasing old entires? Hard?
        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder
                .essentials
                .memory_translation_table
                .remap_read_memory(memory_handle, address_space, address_range);
        }

        (self, memory_handle)
    }

    pub fn insert_write_memory<M: WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_write_memory(callback);

        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder
                .essentials
                .memory_translation_table
                .remap_write_memory(memory_handle, address_space, address_range);
        }

        (self, memory_handle)
    }

    pub fn insert_memory<M: ReadMemory + WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_memory(callback);

        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder
                .essentials
                .memory_translation_table
                .remap_memory(memory_handle, address_space, address_range);
        }

        (self, memory_handle)
    }
}
