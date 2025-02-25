use ines::INes;
use mapper::Mapper;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
};
use multiemu_rom::{id::RomId, manager::RomRequirement};
use std::{
    io::{Cursor, Read},
    sync::Arc,
};

pub mod ines;
mod mapper;

pub struct NesCartidge {
    rom: Vec<u8>,
}

impl Component for NesCartidge {}

#[derive(Debug)]
pub struct NesCartidgeConfig {
    pub rom: RomId,
}

impl FromConfig for NesCartidge {
    type Config = NesCartidgeConfig;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
    ) {
        let mut rom_file = essentials
            .rom_manager()
            .open(config.rom, RomRequirement::Required)
            .unwrap();

        let mut rom = Vec::default();
        rom_file.read_to_end(&mut rom).unwrap();

        // Try parsing as a INES rom
        let header = INes::parse(&rom).unwrap();
    }
}
