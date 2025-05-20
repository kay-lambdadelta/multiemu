use cartridge::Atari2600CartridgeConfig;
use codes_iso_3166::part_1::CountryCode;
use gamepad::joystick::Atari2600JoystickConfig;
use multiemu_config::Environment;
use multiemu_definition_misc::{
    memory::mirror::{MirrorMemoryConfig, PermissionSpace},
    mos6532_riot::Mos6532RiotConfig,
};
use multiemu_definition_mos6502::{Mos6502Config, Mos6502Kind};
use multiemu_machine::{MachineFactory, builder::MachineBuilder, display::shader::ShaderCache};
use multiemu_rom::{
    id::RomId,
    manager::{ROM_INFORMATION_TABLE, RomManager},
    system::{AtariSystem, GameSystem},
};
use num::rational::Ratio;
use std::{
    marker::PhantomData,
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};
use strum::Display;
use tia::{
    SupportedRenderApiTia,
    config::TiaConfig,
    region::{Region, ntsc::Ntsc, pal::Pal, secam::Secam},
};

mod cartridge;
mod gamepad;
mod tia;

#[derive(Debug, Clone, Copy, Display, PartialEq, Eq, PartialOrd, Ord)]
enum RegionSelection {
    Ntsc,
    Pal,
    Secam,
}

trait SupportedRenderApiAtari2600: SupportedRenderApiTia {}

impl<A: SupportedRenderApiTia> SupportedRenderApiAtari2600 for A {}

#[derive(Default, Debug)]
pub struct Atari2600;

impl<R: SupportedRenderApiAtari2600> MachineFactory<R> for Atari2600 {
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
    ) -> MachineBuilder<R> {
        let machine = MachineBuilder::new(
            GameSystem::Atari(AtariSystem::Atari2600),
            rom_manager.clone(),
            environment,
            shader_cache,
        );

        assert_eq!(
            user_specified_roms.len(),
            1,
            "Atari 2600 only requires 1 ROM"
        );

        // Atari 2600 CPU only has 13 address lines
        let (machine, cpu_address_space) = machine.insert_address_space("cpu", 13);

        // Extract information on the rom loaded
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

        let region = if rom_info.regions.contains(&CountryCode::US)
            || rom_info.regions.contains(&CountryCode::JP)
        {
            RegionSelection::Ntsc
        } else if rom_info.regions.contains(&CountryCode::FR)
            || rom_info.regions.contains(&CountryCode::SU)
        {
            RegionSelection::Secam
        } else {
            RegionSelection::Pal
        };

        let (machine, _) = machine.insert_component(
            "cartridge",
            Atari2600CartridgeConfig {
                rom: user_specified_roms[0],
                cpu_address_space,
                force_cart_type: None,
            },
        );

        let (machine, _) = machine.insert_component(
            "tia_write_mirror",
            tia_write_register_mirror_ranges().fold(
                MirrorMemoryConfig::default(),
                |config, source_addresses| {
                    config.insert_range(
                        source_addresses,
                        cpu_address_space,
                        0x0000..=0x003f,
                        cpu_address_space,
                        [PermissionSpace::Write],
                    )
                },
            ),
        );

        let (machine, _) = machine.insert_component(
            "tia_read_mirror",
            tia_read_register_mirror_ranges().fold(
                MirrorMemoryConfig::default(),
                |config, source_addresses| {
                    config.insert_range(
                        source_addresses,
                        cpu_address_space,
                        0x0000..=0x000f,
                        cpu_address_space,
                        [PermissionSpace::Read],
                    )
                },
            ),
        );

        // The mirrors.... yipee.........
        let (machine, _) = machine.insert_component(
            "mos6532_riot_mirrors",
            riot_register_mirror_ranges()
                .chain(riot_ram_mirror_ranges())
                .fold(
                    MirrorMemoryConfig::default(),
                    |config, (source_addresses, destination_addresses)| {
                        config.insert_range(
                            source_addresses,
                            cpu_address_space,
                            destination_addresses,
                            cpu_address_space,
                            [PermissionSpace::Read, PermissionSpace::Write],
                        )
                    },
                ),
        );

        let (machine, mos6532_riot) = match region {
            RegionSelection::Ntsc => {
                let (machine, cpu) = machine.insert_component(
                    "mos_6502",
                    Mos6502Config {
                        frequency: Ntsc::frequency() / Ratio::from_integer(3),
                        kind: Mos6502Kind::M6507,
                        assigned_address_space: cpu_address_space,
                        broken_ror: false,
                    },
                );

                let (machine, mos6532_riot) = machine.insert_component(
                    "mos6532_riot",
                    Mos6532RiotConfig {
                        frequency: Ntsc::frequency() / Ratio::from_integer(3),
                        ram_assigned_address: 0x080,
                        registers_assigned_address: 0x280,
                        assigned_address_space: cpu_address_space,
                    },
                );

                let (machine, _) = machine.insert_component(
                    "tia",
                    TiaConfig::<Ntsc> {
                        cpu,
                        cpu_address_space,
                        _phantom: PhantomData,
                    },
                );

                (machine, mos6532_riot)
            }
            RegionSelection::Pal => {
                let (machine, cpu) = machine.insert_component(
                    "mos_6502",
                    Mos6502Config {
                        frequency: Pal::frequency() / Ratio::from_integer(3),
                        kind: Mos6502Kind::M6507,
                        assigned_address_space: cpu_address_space,
                        broken_ror: false,
                    },
                );

                let (machine, mos6532_riot) = machine.insert_component(
                    "mos6532_riot",
                    Mos6532RiotConfig {
                        frequency: Pal::frequency() / Ratio::from_integer(3),
                        ram_assigned_address: 0x080,
                        registers_assigned_address: 0x280,
                        assigned_address_space: cpu_address_space,
                    },
                );

                let (machine, _) = machine.insert_component(
                    "tia",
                    TiaConfig::<Pal> {
                        cpu,
                        cpu_address_space,
                        _phantom: PhantomData,
                    },
                );

                (machine, mos6532_riot)
            }
            RegionSelection::Secam => {
                let (machine, cpu) = machine.insert_component(
                    "mos_6502",
                    Mos6502Config {
                        frequency: Secam::frequency() / Ratio::from_integer(3),
                        kind: Mos6502Kind::M6507,
                        assigned_address_space: cpu_address_space,
                        broken_ror: false,
                    },
                );

                let (machine, mos6532_riot) = machine.insert_component(
                    "mos6532_riot",
                    Mos6532RiotConfig {
                        frequency: Secam::frequency() / Ratio::from_integer(3),
                        ram_assigned_address: 0x080,
                        registers_assigned_address: 0x280,
                        assigned_address_space: cpu_address_space,
                    },
                );

                let (machine, _) = machine.insert_component(
                    "tia",
                    TiaConfig::<Secam> {
                        cpu,
                        cpu_address_space,
                        _phantom: PhantomData,
                    },
                );

                (machine, mos6532_riot)
            }
        };

        let (machine, _) =
            machine.insert_component("joystick", Atari2600JoystickConfig { mos6532_riot });

        machine
    }
}

// These three functions hardcode mirror addresses instead of trying to mechanically replicate partial address decoding
// Which would be difficult, painful, and require inefficient changes to the memory translation table

fn tia_read_register_mirror_ranges() -> impl Iterator<Item = RangeInclusive<usize>> {
    (1..64).map(|i| {
        let base = i * 0x10;
        base..=base + 0x0f
    })
}

fn tia_write_register_mirror_ranges() -> impl Iterator<Item = RangeInclusive<usize>> {
    (1..32).map(|i| {
        let base = i * 0x40;
        base..=base + 0x3f
    })
}

fn riot_register_mirror_ranges()
-> impl Iterator<Item = (RangeInclusive<usize>, RangeInclusive<usize>)> {
    (1..16).map(|i| {
        let base = 0x280 + i * 0x08;
        (base..=base + 0x03, 0x280..=0x283)
    })
}

fn riot_ram_mirror_ranges() -> impl Iterator<Item = (RangeInclusive<usize>, RangeInclusive<usize>)>
{
    [0x0180, 0x0480, 0x0580, 0x0880, 0x0980, 0x0c80, 0x0d80]
        .into_iter()
        .map(|range| ((range..=range + 0x7f), 0x80..=0xff))
}
