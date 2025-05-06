use banking::BankingCartridgeMemoryCallback;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::AddressSpaceHandle,
};
use multiemu_rom::{id::RomId, manager::RomRequirement};
use raw::RawCartridgeMemoryCallback;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod banking;
mod raw;

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum CartType {
    #[default]
    Raw,
    Banking1k,
    Banking2k,
    Banking4k,
}

pub struct Atari2600Cartridge {}

#[derive(Debug)]
pub struct Atari2600CartridgeConfig {
    pub rom: RomId,
    pub cpu_address_space: AddressSpaceHandle,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Atari2600CartridgeQuirks {
    pub force_cart_type: Option<CartType>,
}

impl Component for Atari2600Cartridge {}

impl FromConfig for Atari2600Cartridge {
    type Config = Atari2600CartridgeConfig;
    type Quirks = Atari2600CartridgeQuirks;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        quirks: Self::Quirks,
    ) {
        let mut rom = essentials
            .rom_manager
            .open(
                config.rom,
                RomRequirement::Required,
                &essentials.environment.read().unwrap().roms_directory,
            )
            .unwrap();
        let mut rom_bytes = Vec::default();
        std::io::copy(&mut rom, &mut rom_bytes).unwrap();

        assert!(rom_bytes.len().is_power_of_two(), "Obviously invalid rom");

        let cart_type = quirks.force_cart_type.unwrap_or_else(|| {
            if rom_bytes.len() <= 0x4000 {
                CartType::Raw
            } else {
                todo!()
            }
        });

        let component_builder = match cart_type {
            CartType::Raw => component_builder.insert_read_memory(
                RawCartridgeMemoryCallback {
                    rom: rom_bytes.try_into().unwrap(),
                },
                [(config.cpu_address_space, 0x1000..=0x1fff)],
            ),
            CartType::Banking1k => component_builder.insert_read_memory(
                BankingCartridgeMemoryCallback::<0x1000>::new(rom_bytes),
                [(config.cpu_address_space, 0x1000..=0x1fff)],
            ),
            CartType::Banking2k => component_builder.insert_read_memory(
                BankingCartridgeMemoryCallback::<0x2000>::new(rom_bytes),
                [(config.cpu_address_space, 0x1000..=0x1fff)],
            ),
            CartType::Banking4k => component_builder.insert_read_memory(
                BankingCartridgeMemoryCallback::<0x4000>::new(rom_bytes),
                [(config.cpu_address_space, 0x1000..=0x1fff)],
            ),
        };

        component_builder.build_global(Self {});
    }
}
