use crate::{INes, cartridge::ines::RomType};
use multiemu::{
    component::{BuildError, Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
    rom::RomId,
};
use multiemu_definition_misc::memory::{mirror::MirrorMemoryConfig, rom::RomMemoryConfig};

#[derive(Debug)]
pub struct Mapper000;

impl Component for Mapper000 {}

#[derive(Debug)]
pub struct Mapper000Config<'a> {
    pub ines: &'a INes,
    pub rom_id: RomId,
    pub cpu_address_space: AddressSpaceId,
    pub ppu_address_space: AddressSpaceId,
}

impl<'a, P: Platform> ComponentConfig<P> for Mapper000Config<'a> {
    type Component = Mapper000;

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, BuildError> {
        let component_builder = match self.ines.prg_bank_count() {
            // NROM-128
            1 => {
                let (component_builder, _) = component_builder.insert_child_component(
                    "prg",
                    RomMemoryConfig {
                        rom: self.rom_id,
                        assigned_address_space: self.cpu_address_space,
                        assigned_range: 0x8000..=0xbfff,
                        rom_range: self.ines.roms.get(&RomType::Prg).unwrap().clone(),
                    },
                );

                let (component_builder, _) = component_builder.insert_child_component(
                    "prg-mirror",
                    MirrorMemoryConfig {
                        source_address_space: self.cpu_address_space,
                        source_addresses: 0xc000..=0xffff,
                        destination_address_space: self.cpu_address_space,
                        destination_addresses: 0x8000..=0xbfff,
                        readable: true,
                        writable: false,
                    },
                );

                component_builder
            }
            // NROM-256
            2 => {
                let (component_builder, _) = component_builder.insert_child_component(
                    "prg",
                    RomMemoryConfig {
                        rom: self.rom_id,
                        assigned_address_space: self.cpu_address_space,
                        assigned_range: 0x8000..=0xffff,
                        rom_range: self.ines.roms.get(&RomType::Prg).unwrap().clone(),
                    },
                );

                component_builder
            }
            _ => {
                panic!("Unsupported PRG ROM size for NROM mapper: {:?}", self.ines);
            }
        };

        component_builder.insert_child_component(
            "chr",
            RomMemoryConfig {
                rom: self.rom_id,
                assigned_address_space: self.ppu_address_space,
                assigned_range: 0x0000..=0x1fff,
                rom_range: self.ines.roms.get(&RomType::Chr).unwrap().clone(),
            },
        );

        Ok(Mapper000)
    }
}
