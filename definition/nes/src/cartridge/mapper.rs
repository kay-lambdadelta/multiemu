use super::NesCartridge;
use crate::INes;
use multiemu_machine::{
    builder::ComponentBuilder,
    memory::{
        AddressSpaceHandle,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use rangemap::RangeInclusiveMap;
use std::sync::Arc;

#[derive(Debug)]
struct MemoryCallbacks {
    bus_conflict: bool,
}

impl ReadMemory for MemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
    }
}

impl WriteMemory for MemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        let original_data = buffer[0];
        let mut data = buffer[0];

        // https://www.nesdev.org/wiki/Bus_conflict
        if self.bus_conflict {
            data &= 1;

            if original_data != data {
                tracing::warn!("Bus conflict affected write to register {}", address);
            }
        }
    }
}

pub fn construct_mapper(
    component_builder: ComponentBuilder<NesCartridge>,
    ines: Arc<INes>,
    cpu_address_space: AddressSpaceHandle,
    ppu_address_space: AddressSpaceHandle,
) -> ComponentBuilder<NesCartridge> {
    match ines.mapper {
        000 => {}
        _ => unimplemented!(),
    }

    let memory_callbacks = Arc::new(MemoryCallbacks {
        bus_conflict: false,
    });

    component_builder.insert_rw_memory::<MemoryCallbacks>(
        memory_callbacks.clone(),
        [(cpu_address_space, 0x8000..=0xffff)],
    )
}
