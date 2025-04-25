use cartridge::{Atari2600Cartridge, Atari2600CartridgeConfig};
use codes_iso_3166::part_1::CountryCode;
use gamepad::joystick::{Atari2600Joystick, Atari2600JoystickConfig};
use multiemu_config::Environment;
use multiemu_definition_m6502::{M6502, M6502Config, M6502Kind};
use multiemu_definition_misc::m6532_riot::{M6532Riot, M6532RiotConfig};
use multiemu_machine::{
    builder::MachineBuilder, display::shader::ShaderCache, memory::AddressSpaceId,
};
use multiemu_rom::{
    id::RomId,
    manager::{ROM_INFORMATION_TABLE, RomManager},
    system::{AtariSystem, GameSystem},
};
use num::rational::Ratio;
use std::sync::{Arc, RwLock};
use tia::{
    Tia, TiaConfig,
    region::{Region, ntsc::Ntsc, pal::Pal},
};

mod cartridge;
mod gamepad;
pub mod tia;

const CPU_ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

pub fn manifest(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: Arc<ShaderCache>,
) -> MachineBuilder {
    let machine = MachineBuilder::new(
        GameSystem::Atari(AtariSystem::Atari2600),
        rom_manager.clone(),
        environment,
        shader_cache,
    );

    let machine = machine.insert_address_space(CPU_ADDRESS_SPACE, 16);

    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
    let table = database_transaction
        .open_multimap_table(ROM_INFORMATION_TABLE)
        .unwrap();
    let rom_info = table
        .get(&user_specified_roms[0])
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .value();

    if rom_info.regions.contains(&CountryCode::US) || rom_info.regions.contains(&CountryCode::JP) {
        tracing::info!("Using NTSC region");

        machine
            .insert_component::<M6502>(
                "mos_6502",
                M6502Config {
                    frequency: Ntsc::frequency() / Ratio::from_integer(3),
                    kind: M6502Kind::M6507,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                },
            )
            .insert_component::<M6532Riot>(
                "m6532_riot",
                M6532RiotConfig {
                    frequency: Ntsc::frequency() / Ratio::from_integer(3),
                    ram_assigned_address: 0x080,
                    registers_assigned_address: 0x280,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                },
            )
            .insert_component::<Atari2600Joystick>(
                "joystick",
                Atari2600JoystickConfig {
                    m6532_riot: "m6532_riot".into(),
                },
            )
            .insert_component::<Atari2600Cartridge>(
                "cartridge",
                Atari2600CartridgeConfig {
                    rom: user_specified_roms[0],
                },
            )
            .insert_component::<Tia<Ntsc>>(
                "tia_(ntsc)",
                TiaConfig {
                    processor_name: "mos_6502",
                },
            )
    } else {
        tracing::info!("Using PAL region");

        machine
            .insert_component::<M6502>(
                "mos_6502",
                M6502Config {
                    frequency: Pal::frequency() / Ratio::from_integer(3),
                    kind: M6502Kind::M6507,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                },
            )
            .insert_component::<M6532Riot>(
                "m6532_riot",
                M6532RiotConfig {
                    frequency: Pal::frequency() / Ratio::from_integer(3),
                    ram_assigned_address: 0x080,
                    registers_assigned_address: 0x280,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                },
            )
            .insert_component::<Atari2600Joystick>(
                "joystick",
                Atari2600JoystickConfig {
                    m6532_riot: "m6532_riot".into(),
                },
            )
            .insert_component::<Atari2600Cartridge>(
                "cartridge",
                Atari2600CartridgeConfig {
                    rom: user_specified_roms[0],
                },
            )
            .insert_component::<Tia<Pal>>(
                "tia_(pal)",
                TiaConfig {
                    processor_name: "mos_6502",
                },
            )
    }
}
