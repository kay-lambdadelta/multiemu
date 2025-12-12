use bytes::Bytes;
use multiemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
};
use serde::{Deserialize, Serialize};

use crate::cartridge::mapper::{Mapper, mmc1::Mmc1Config, nrom::NRomConfig};

pub mod ines;
pub mod mapper;

#[derive(Debug)]
pub struct NesCartridge;

impl Component for NesCartridge {}

#[derive(Debug)]
pub struct NesCartridgeConfig {
    pub cpu_address_space: AddressSpaceId,
    pub ppu_address_space: AddressSpaceId,
    pub chr: Bytes,
    pub prg: Bytes,
    pub mapper: Mapper,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct NesCartridgeQuirks {
    pub force_mapper: Mapper,
}

impl<P: Platform> ComponentConfig<P> for NesCartridgeConfig {
    type Component = NesCartridge;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        match self.mapper {
            Mapper::NRom => {
                component_builder
                    .insert_child_component("nrom", NRomConfig { config: &self })
                    .0
            }
            Mapper::Mmc1 => {
                component_builder
                    .insert_child_component("mmc1", Mmc1Config { config: &self })
                    .0
            }
            _ => {
                unreachable!("Unsupported mapper {:?}", self.mapper);
            }
        };

        Ok(NesCartridge)
    }
}
