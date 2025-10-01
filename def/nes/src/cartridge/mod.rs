use crate::cartridge::mapper::mapper_000::Mapper000Config;
use ines::INes;
use multiemu::{
    component::{BuildError, Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
    rom::{RomId, RomRequirement},
};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, Read};

pub mod ines;
pub mod mapper;

#[derive(Debug)]
pub struct NesCartridge {
    rom: INes,
}

impl NesCartridge {
    pub fn rom(&self) -> INes {
        self.rom.clone()
    }
}

impl Component for NesCartridge {}

#[derive(Debug)]
pub struct NesCartridgeConfig {
    pub cpu_address_space: AddressSpaceId,
    pub ppu_address_space: AddressSpaceId,
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
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let rom_manager = component_builder.rom_manager();

        let mut rom_file = BufReader::new(
            rom_manager
                .open(self.rom, RomRequirement::Required)
                .unwrap(),
        );

        let mut header = [0; 16];
        rom_file.read_exact(&mut header).unwrap();

        // Try parsing as a INES rom
        let ines = INes::parse(header).unwrap();

        tracing::info!("Loaded INES ROM: {:?}", ines);

        let component_builder = match ines.mapper {
            000 => {
                component_builder
                    .insert_child_component(
                        "mapper_000",
                        Mapper000Config {
                            ines: &ines,
                            rom_id: self.rom,
                            cpu_address_space: self.cpu_address_space,
                            ppu_address_space: self.ppu_address_space,
                        },
                    )
                    .0
            }
            _ => {
                unreachable!("Unsupported mapper {}", ines.mapper);
            }
        };

        component_builder.build(NesCartridge { rom: ines });

        Ok(())
    }
}
