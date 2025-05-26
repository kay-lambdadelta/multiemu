use crate::INes;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::Component,
    display::backend::RenderApi,
    memory::{
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryOperationError, ReadMemoryRecord, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
};
use std::sync::Arc;

#[derive(Debug)]
struct NesCartidgeMemoryCallbacks {
    bus_conflict: bool,
}

impl ReadMemory for NesCartidgeMemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        Ok(())
    }
}

impl WriteMemory for NesCartidgeMemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let original_data = buffer[0];
        let mut data = buffer[0];

        // https://www.nesdev.org/wiki/Bus_conflict
        if self.bus_conflict {
            data &= 1;

            if original_data != data {
                tracing::warn!("Bus conflict affected write to register {}", address);
            }
        }

        Ok(())
    }
}

pub fn construct_mapper<R: RenderApi, C: Component>(
    ines: Arc<INes>,
    cpu_address_space: AddressSpaceHandle,
    ppu_address_space: AddressSpaceHandle,
    component_builder: ComponentBuilder<R, C>,
) -> ComponentBuilder<R, C> {
    match ines.mapper {
        000 => {}
        _ => unimplemented!(),
    }

    let memory_callbacks = Arc::new(NesCartidgeMemoryCallbacks {
        bus_conflict: false,
    });

    component_builder
        .insert_memory(
            memory_callbacks.clone(),
            [(cpu_address_space, 0x8000..=0xffff)],
        )
        .0
}
