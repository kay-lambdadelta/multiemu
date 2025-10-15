use crate::cartridge::mapper::{mmc1::Mmc1Config, nrom::NRomConfig};
use ines::INes;
use multiemu_base::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
    program::{RomId, RomRequirement},
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
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let program_manager = component_builder.program_manager();

        let mut rom_file = BufReader::new(
            program_manager
                .open(self.rom, RomRequirement::Required)
                .unwrap(),
        );

        let mut header = [0; 16];
        rom_file.read_exact(&mut header).unwrap();

        // Try parsing as a INES rom
        let ines = INes::parse(header).unwrap();

        tracing::info!("Loaded INES ROM: {:?}", ines);

        #[allow(clippy::zero_prefixed_literal)]
        match ines.mapper {
            000 => {
                component_builder
                    .insert_child_component(
                        "nrom",
                        NRomConfig {
                            ines: &ines,
                            rom_id: self.rom,
                            cpu_address_space: self.cpu_address_space,
                            ppu_address_space: self.ppu_address_space,
                        },
                    )
                    .0
            }
            001 | 155 => {
                component_builder
                    .insert_child_component(
                        "mmc1",
                        Mmc1Config {
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

        Ok(NesCartridge { rom: ines })
    }
}
