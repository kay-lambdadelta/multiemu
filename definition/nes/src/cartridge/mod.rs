use ines::INes;
use multiemu_rom::{RomId, RomRequirement};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, ComponentRef},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, ReadMemoryRecord, WriteMemoryRecord,
    },
    platform::Platform,
};
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::Arc};

pub mod ines;

#[derive(Debug)]
pub struct NesCartridge {
    rom: Arc<INes>,
    bus_conflict: bool,
}

impl NesCartridge {
    pub fn rom(&self) -> Arc<INes> {
        self.rom.clone()
    }
}

impl Component for NesCartridge {
    fn read_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        Ok(())
    }

    fn write_memory(
        &self,
        address: Address,
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

#[derive(Debug)]
pub struct NesCartridgeConfig {
    pub cpu_address_space: AddressSpaceHandle,
    pub ppu_address_space: AddressSpaceHandle,
    pub rom: RomId,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct NesCartridgeQuirks {
    pub force_mapper: u8,
}

impl<P: Platform> ComponentConfig<P> for NesCartridgeConfig {
    type Component = NesCartridge;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) {
        let essentials = component_builder.essentials();

        let mut rom_file = essentials
            .rom_manager
            .open(self.rom, RomRequirement::Required)
            .unwrap();

        let mut rom = Vec::default();
        rom_file.read_to_end(&mut rom).unwrap();

        // Try parsing as a INES rom
        let ines = Arc::new(INes::parse(&rom).unwrap());
        tracing::debug!("Parsed ROM as {:#?}", ines);

        let component_builder =
            component_builder.map_memory([(self.cpu_address_space, 0x8000..=0xffff)]);

        component_builder.build_global(NesCartridge {
            rom: ines,
            bus_conflict: false,
        })
    }
}
