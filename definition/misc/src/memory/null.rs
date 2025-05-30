use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::{
        Address,
        callbacks::{Memory, ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryHandle, MemoryOperationError, ReadMemoryRecord, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
};
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct NullMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    // Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<Address>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceHandle,
}

impl<R: RenderApi> ComponentConfig<R> for NullMemoryConfig {
    type Component = NullMemory;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        let (component_builder, memory_handle) = match (self.readable, self.writable) {
            (true, true) => component_builder.insert_memory(
                MemoryCallbacks,
                [(self.assigned_address_space, self.assigned_range)],
            ),
            (true, false) => component_builder.insert_read_memory(
                MemoryCallbacks,
                [(self.assigned_address_space, self.assigned_range)],
            ),
            (false, true) => component_builder.insert_write_memory(
                MemoryCallbacks,
                [(self.assigned_address_space, self.assigned_range)],
            ),
            (false, false) => {
                panic!("Huh?");
            }
        };

        component_builder.build_global(NullMemory { memory_handle });
    }
}

#[derive(Debug)]
/// Always denies accesses, if you need this for some reason it exists
pub struct NullMemory {
    pub memory_handle: MemoryHandle,
}

impl Component for NullMemory {}

#[derive(Debug)]
struct MemoryCallbacks;

impl Memory for MemoryCallbacks {}

impl ReadMemory for MemoryCallbacks {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        Err(RangeInclusiveMap::from_iter([(
            address..=(address + (buffer.len() - 1)),
            ReadMemoryRecord::Denied,
        )])
        .into())
    }
}

impl WriteMemory for MemoryCallbacks {
    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        Err(RangeInclusiveMap::from_iter([(
            address..=(address + (buffer.len() - 1)),
            WriteMemoryRecord::Denied,
        )])
        .into())
    }
}
