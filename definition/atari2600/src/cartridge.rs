use multiemu_rom::{RomId, RomRequirement};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentRef},
    memory::{Address, AddressSpaceHandle, MemoryOperationError, ReadMemoryRecord},
    platform::Platform,
};
use multiemu_save::ComponentSave;
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
    pub cpu_address_space: AddressSpaceHandle,
    pub force_cart_type: Option<CartType>,
}

impl Component for Atari2600Cartridge {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        match self.cart_type {
            CartType::Raw => {
                let adjusted_offset = address - 0x1000;
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
        _component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
        _save: Option<&ComponentSave>,
    ) -> Result<(), BuildError> {
        let essentials = component_builder.essentials();

        let mut rom = essentials
            .rom_manager
            .open(self.rom, RomRequirement::Required)
            .unwrap();
        let mut rom_bytes = Vec::default();
        std::io::copy(&mut rom, &mut rom_bytes).unwrap();

        assert!(rom_bytes.len().is_power_of_two(), "Obviously invalid rom");

        let cart_type = self.force_cart_type.unwrap_or_else(|| {
            if rom_bytes.len() <= 0x4000 {
                CartType::Raw
            } else {
                todo!()
            }
        });

        component_builder
            .map_memory_read([(self.cpu_address_space, 0x1000..=0x1fff)])
            .build_global(Atari2600Cartridge {
                cart_type,
                rom: rom_bytes,
            });

        Ok(())
    }
}
