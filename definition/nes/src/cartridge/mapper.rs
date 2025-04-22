use super::NesCartridge;
use crate::{CPU_ADDRESS_SPACE, INes};
use multiemu_machine::{
    builder::ComponentBuilder,
    memory::{
        AddressSpaceId,
        callbacks::Memory,
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use rangemap::RangeInclusiveMap;
use std::sync::Arc;

#[derive(Debug)]
struct MemoryCallbacks {
    bus_conflict: bool,
}

impl Memory for MemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
    }

    fn write_memory(
        &self,
        address: usize,
        address_space: AddressSpaceId,
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
) -> ComponentBuilder<NesCartridge> {
    match ines.mapper {
        000 => {}
        _ => unimplemented!(),
    }

    let memory_callbacks = Arc::new(MemoryCallbacks {
        bus_conflict: false,
    });

    component_builder.insert_memory(
        [(0x8000..=0xffff, CPU_ADDRESS_SPACE)],
        memory_callbacks.clone(),
    )
}
