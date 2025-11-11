use crate::{INes, cartridge::ines::RomType};
use multiemu_definition_misc::memory::rom::RomMemoryConfig;
use multiemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
    program::RomId,
};

#[derive(Debug)]
pub struct NRom;

impl Component for NRom {}

#[derive(Debug)]
pub struct NRomConfig<'a> {
    pub ines: &'a INes,
    pub rom_id: RomId,
    pub cpu_address_space: AddressSpaceId,
    pub ppu_address_space: AddressSpaceId,
}

impl<P: Platform> ComponentConfig<P> for NRomConfig<'_> {
    type Component = NRom;

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
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

                component_builder.memory_mirror_map_read(
                    self.cpu_address_space,
                    0xc000..=0xffff,
                    0x8000..=0xbfff,
                )
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

        Ok(NRom)
    }
}
