use cartridge::{Atari2600Cartridge, Atari2600CartridgeConfig};
use codes_iso_3166::part_1::CountryCode;
use gamepad::joystick::{Atari2600Joystick, Atari2600JoystickConfig};
use multiemu_config::Environment;
use multiemu_definition_m6502::{M6502, M6502Config, M6502Kind};
use multiemu_definition_misc::{
    m6532_riot::{M6532Riot, M6532RiotConfig},
    memory::mirror::{MirrorMemory, MirrorMemoryConfig},
};
use multiemu_machine::{
    builder::MachineBuilder, display::shader::ShaderCache, memory::AddressSpaceId,
};
use multiemu_rom::{
    id::RomId,
    manager::{ROM_INFORMATION_TABLE, RomManager},
    system::{AtariSystem, GameSystem},
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use std::{
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};
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

    let machine = machine.insert_address_space(CPU_ADDRESS_SPACE, 13);

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
            // The mirrors.... yipee.........
            .insert_component::<MirrorMemory>(
                "m6532_riot_mirrors",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                    assigned_ranges: RangeInclusiveMap::from_iter(
                        riot_register_mirror_ranges().chain(riot_ram_mirror_ranges()),
                    ),
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
            .insert_component::<MirrorMemory>(
                "tia_mirror",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                    assigned_ranges: RangeInclusiveMap::from_iter(tia_register_mirror_ranges()),
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
            .insert_component::<MirrorMemory>(
                "m6532_riot_mirrors",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                    assigned_ranges: RangeInclusiveMap::from_iter(
                        riot_register_mirror_ranges().chain(riot_ram_mirror_ranges()),
                    ),
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
            .insert_component::<MirrorMemory>(
                "tia_mirror",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    assigned_address_space: CPU_ADDRESS_SPACE,
                    assigned_ranges: RangeInclusiveMap::from_iter(tia_register_mirror_ranges()),
                },
            )
    }
}
fn tia_register_mirror_ranges() -> impl Iterator<Item = (RangeInclusive<usize>, usize)> {
    [
        0x0040, 0x0100, 0x0140, 0x0200, 0x0240, 0x0300, 0x0340, 0x0400, 0x0440, 0x0500, 0x0540,
        0x0600, 0x0640, 0x0700, 0x0740, 0x0800, 0x0840, 0x0900, 0x0940, 0x0a00, 0x0a40, 0x0b00,
        0x0b40, 0x0c00, 0x0c40, 0x0d00, 0x0d40, 0x0e00, 0x0e40, 0x0f00, 0x0f40,
    ]
    .into_iter()
    .map(|addr| (addr..=addr + 0x3f, 0x0000))
}

fn riot_register_mirror_ranges() -> impl Iterator<Item = (RangeInclusive<usize>, usize)> {
    [
        0x02a0, 0x02c0, 0x02e0, 0x0380, 0x03a0, 0x03c0, 0x03e0, 0x0680, 0x06a0, 0x06c0, 0x06e0,
        0x0780, 0x07a0, 0x07c0, 0x07e0, 0x0a80, 0x0aa0, 0x0ac0, 0x0ae0, 0x0b80, 0x0ba0, 0x0bc0,
        0x0be0, 0x0e80, 0x0ea0, 0x0ec0, 0x0ee0, 0x0f80, 0x0fa0, 0x0fc0, 0x0fe0,
    ]
    .into_iter()
    .map(|addr| (addr..=addr + 0x1f, 0x280))
}

fn riot_ram_mirror_ranges() -> impl Iterator<Item = (RangeInclusive<usize>, usize)> {
    [0x0180, 0x0480, 0x0580, 0x0880, 0x0980, 0x0c80, 0x0d80]
        .into_iter()
        .map(|range| ((range..=range + 0x7f), 0x80))
}
