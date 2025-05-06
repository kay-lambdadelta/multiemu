use ines::INes;
use mapper::construct_mapper;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::AddressSpaceHandle,
};
use multiemu_rom::{id::RomId, manager::RomRequirement};
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::Arc};

pub mod ines;
mod mapper;

pub struct NesCartridge {
    rom: Arc<INes>,
}

impl NesCartridge {
    pub fn rom(&self) -> Arc<INes> {
        self.rom.clone()
    }
}

impl Component for NesCartridge {}

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

impl FromConfig for NesCartridge {
    type Config = NesCartridgeConfig;
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let mut rom_file = essentials
            .rom_manager
            .open(
                config.rom,
                RomRequirement::Required,
                &essentials.environment.read().unwrap().roms_directory,
            )
            .unwrap();

        let mut rom = Vec::default();
        rom_file.read_to_end(&mut rom).unwrap();

        // Try parsing as a INES rom
        let ines = Arc::new(INes::parse(&rom).unwrap());
        tracing::debug!("Parsed ROM as {:#?}", ines);

        let component_builder = construct_mapper(
            component_builder,
            ines.clone(),
            config.cpu_address_space,
            config.ppu_address_space,
        );
        component_builder.build_global(Self { rom: ines });
    }
}
