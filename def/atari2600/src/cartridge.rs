use multiemu::{
    component::{BuildError, Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, ReadMemoryError},
    platform::Platform,
    rom::{RomId, RomRequirement},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum CartType {
    #[default]
    Raw,
    Banking1k,
    Banking2k,
    Banking4k,
}

#[derive(Debug)]
pub struct Atari2600Cartridge {
    rom: Vec<u8>,
    cart_type: CartType,
}

#[derive(Debug)]
pub struct Atari2600CartridgeConfig {
    pub rom: RomId,
    pub cpu_address_space: AddressSpaceId,
    pub force_cart_type: Option<CartType>,
}

impl Component for Atari2600Cartridge {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        match self.cart_type {
            CartType::Raw => {
                let adjusted_offset = (address - 0x1000) % self.rom.len();
                buffer.copy_from_slice(
                    &self.rom[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))],
                );

                Ok(())
            }
            CartType::Banking1k => todo!(),
            CartType::Banking2k => todo!(),
            CartType::Banking4k => todo!(),
        }
    }
}

impl<P: Platform> ComponentConfig<P> for Atari2600CartridgeConfig {
    type Component = Atari2600Cartridge;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, BuildError> {
        let rom_manager = component_builder.rom_manager();

        let mut rom = rom_manager
            .open(self.rom, RomRequirement::Required)
            .unwrap();
        let mut rom_bytes = Vec::default();
        std::io::copy(&mut rom, &mut rom_bytes).unwrap();

        assert!(rom_bytes.len().is_power_of_two(), "Obviously invalid rom");

        let cart_type = self.force_cart_type.unwrap_or_else(|| {
            if rom_bytes.len() <= 0x1000 {
                CartType::Raw
            } else {
                todo!()
            }
        });

        component_builder.memory_map_read(self.cpu_address_space, 0x1000..=0x1fff);

        Ok(Atari2600Cartridge {
            cart_type,
            rom: rom_bytes,
        })
    }
}
