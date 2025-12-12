use multiemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    platform::Platform,
};

use crate::cartridge::NesCartridgeConfig;

#[derive(Debug)]
pub struct NRom;

impl Component for NRom {}

#[derive(Debug)]
pub struct NRomConfig<'a> {
    pub config: &'a NesCartridgeConfig,
}

impl<P: Platform> ComponentConfig<P> for NRomConfig<'_> {
    type Component = NRom;

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let prg_bank_count = self.config.prg.len() / (16 * 1024);

        let component_builder = match prg_bank_count {
            // NROM-128
            1 => {
                let (component_builder, prg) = component_builder.memory_register_buffer(
                    self.config.cpu_address_space,
                    "prg",
                    self.config.prg.clone(),
                );

                let component_builder = component_builder.memory_map_buffer_read(
                    self.config.cpu_address_space,
                    0x8000..=0xbfff,
                    &prg,
                );

                component_builder.memory_mirror_map_read(
                    self.config.cpu_address_space,
                    0xc000..=0xffff,
                    0x8000..=0xbfff,
                )
            }
            // NROM-256
            2 => {
                let (component_builder, prg) = component_builder.memory_register_buffer(
                    self.config.cpu_address_space,
                    "prg",
                    self.config.prg.clone(),
                );

                component_builder.memory_map_buffer_read(
                    self.config.cpu_address_space,
                    0x8000..=0xffff,
                    &prg,
                )
            }
            _ => {
                panic!("Unsupported PRG ROM size for NROM mapper");
            }
        };

        let (component_builder, chr) = component_builder.memory_register_buffer(
            self.config.ppu_address_space,
            "chr",
            self.config.chr.clone(),
        );

        component_builder.memory_map_buffer_read(
            self.config.ppu_address_space,
            0x0000..=0x1fff,
            &chr,
        );

        Ok(NRom)
    }
}
