use cartridge::Atari2600CartridgeConfig;
use gamepad::joystick::Atari2600JoystickConfig;
use multiemu_definition_misc::mos6532_riot::Mos6532RiotConfig;
use multiemu_definition_mos6502::{Mos6502Config, Mos6502Kind};
use multiemu_runtime::{
    component::ComponentPath,
    machine::{MachineFactory, builder::MachineBuilder},
    memory::{Address, AddressSpaceId},
    platform::Platform,
    program::Filesystem,
};
use std::{marker::PhantomData, ops::RangeInclusive};
use strum::Display;
use tia::{
    config::TiaConfig,
    region::{Region, ntsc::Ntsc, pal::Pal, secam::Secam},
};

use crate::tia::SupportedGraphicsApiTia;

mod cartridge;
mod gamepad;
mod tia;

#[derive(Debug, Clone, Copy, Display, PartialEq, Eq, PartialOrd, Ord)]
enum RegionSelection {
    Ntsc,
    Pal,
    Secam,
}

#[derive(Default, Debug)]
pub struct Atari2600;

impl<P: Platform<GraphicsApi: SupportedGraphicsApiTia>> MachineFactory<P> for Atari2600 {
    fn construct(&self, machine: MachineBuilder<P>) -> MachineBuilder<P> {
        // Atari 2600 CPU only has 13 address lines
        let (machine, cpu_address_space) = machine.insert_address_space(13);
        // For now, assume all games are ntsc
        let region = RegionSelection::Ntsc;

        let Filesystem::Single { rom_id, .. } =
            machine.program_specification().unwrap().info.filesystem()
        else {
            panic!("No atari 2600 game has a structured filesystem")
        };
        let rom = *rom_id;

        let (mut machine, _) = machine.insert_component(
            "cartridge",
            Atari2600CartridgeConfig {
                rom,
                cpu_address_space,
                force_cart_type: None,
            },
        );

        for source_addresses in tia_register_mirror_ranges() {
            machine =
                machine.memory_map_mirror(cpu_address_space, source_addresses, 0x0000..=0x003f);
        }

        for source_addresses in riot_register_mirror_ranges() {
            machine = machine.memory_map_mirror(cpu_address_space, source_addresses, 0x280..=0x29f);
        }

        for source_addresses in riot_ram_mirror_ranges() {
            machine = machine.memory_map_mirror(cpu_address_space, source_addresses, 0x80..=0xff);
        }

        let (machine, mos6532_riot) = match region {
            RegionSelection::Ntsc => common::<Ntsc, _>(cpu_address_space, machine),
            RegionSelection::Pal => common::<Pal, _>(cpu_address_space, machine),
            RegionSelection::Secam => common::<Secam, _>(cpu_address_space, machine),
        };

        let (machine, _) =
            machine.insert_component("joystick", Atari2600JoystickConfig { mos6532_riot });

        machine
    }
}

fn common<R: Region, P: Platform<GraphicsApi: SupportedGraphicsApiTia>>(
    cpu_address_space: AddressSpaceId,
    machine: MachineBuilder<P>,
) -> (MachineBuilder<P>, ComponentPath) {
    let (machine, cpu) = machine.insert_component(
        "mos_6502",
        Mos6502Config {
            frequency: R::frequency() / 3,
            kind: Mos6502Kind::Mos6507,
            assigned_address_space: cpu_address_space,
            broken_ror: false,
        },
    );

    let (machine, mos6532_riot) = machine.insert_component(
        "mos6532_riot",
        Mos6532RiotConfig {
            frequency: R::frequency() / 3,
            registers_assigned_address: 0x280,
            ram_assigned_address: 0x80,
            assigned_address_space: cpu_address_space,
        },
    );

    let (machine, _) = machine.insert_component(
        "tia",
        TiaConfig::<R> {
            cpu,
            cpu_address_space,
            _phantom: PhantomData,
        },
    );

    (machine, mos6532_riot)
}

// These three functions hardcode mirror addresses instead of trying to mechanically replicate partial address decoding
// Which would be difficult, painful, and require inefficient changes to the memory translation table

fn tia_register_mirror_ranges() -> impl Iterator<Item = RangeInclusive<Address>> {
    [
        0x0000, 0x0040, 0x0100, 0x0140, 0x0200, 0x0240, 0x0300, 0x0340, 0x0400, 0x0440, 0x0500,
        0x0540, 0x0600, 0x0640, 0x0700, 0x0740, 0x0800, 0x0840, 0x0900, 0x0940, 0x0a00, 0x0a40,
        0x0b00, 0x0b40, 0x0c00, 0x0c40, 0x0d00, 0x0d40, 0x0e00, 0x0e40, 0x0f00, 0x0f40,
    ]
    .into_iter()
    .skip(1)
    .map(|start_range| start_range..=start_range + 0x003f)
}

// 0x280..=0x29f
#[rustfmt::skip]
fn riot_register_mirror_ranges() -> impl Iterator<Item = RangeInclusive<Address>> {
    [
        0x280, 0x2a0, 0x2c0, 0x2e0,
        0x380, 0x3a0, 0x3c0, 0x3e0,
        0x680,  0x6a0, 0x6c0, 0x6e0,
        0x780, 0x7a0, 0x7c0, 0x7e0,
        0xa80, 0xaa0, 0xac0, 0xae0,
        0xb80, 0xba0, 0xbc0, 0xbe0,
        0xe80, 0xea0, 0xec0, 0xee0,
        0xf80, 0xfa0, 0xfc0, 0xfe0,
    ]
    .into_iter()
    .skip(1)
    .map(|range| range..=range + 0x1f)
}

// 0x80..=0xff
fn riot_ram_mirror_ranges() -> impl Iterator<Item = RangeInclusive<Address>> {
    [
        0x080, 0x0180, 0x0480, 0x0580, 0x0880, 0x0980, 0x0c80, 0x0d80,
    ]
    .into_iter()
    .skip(1)
    .map(|range| range..=range + 0x7f)
}
