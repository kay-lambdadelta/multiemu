use cartridge::Atari2600CartridgeConfig;
use codes_iso_3166::part_1::CountryCode;
use gamepad::joystick::Atari2600JoystickConfig;
use multiemu_definition_misc::{
    memory::{
        mirror::MirrorMemoryConfig,
        standard::{StandardMemoryConfig, StandardMemoryInitialContents},
    },
    mos6532_riot::{Mos6532Riot, Mos6532RiotConfig},
};
use multiemu_definition_mos6502::{Mos6502Config, Mos6502Kind};
use multiemu_rom::{ROM_INFORMATION_TABLE, RomId, RomManager};
use multiemu_runtime::{
    builder::MachineBuilder, component::{ComponentId, ComponentRef}, memory::{Address, AddressSpaceHandle}, platform::Platform, MachineFactory
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use std::{marker::PhantomData, ops::RangeInclusive, sync::Arc};
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
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> MachineBuilder<P> {
        let machine =
            MachineBuilder::<P>::new(rom_manager.clone(), sample_rate, main_thread_executor);

        assert_eq!(
            user_specified_roms.len(),
            1,
            "Atari 2600 only requires 1 ROM"
        );

        // Atari 2600 CPU only has 13 address lines
        let (machine, cpu_address_space) = machine.insert_address_space(13);

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

        let mut mirror_component_ids = Vec::default();

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

        let (mut machine, _) = machine.insert_component(
            "cartridge",
            Atari2600CartridgeConfig {
                rom: user_specified_roms[0],
                cpu_address_space,
                force_cart_type: None,
            },
        );

        for (index, source_addresses) in tia_write_register_mirror_ranges().enumerate() {
            let (machine_builder, component) = machine.insert_component(
                &format!("tia_write_mirror_{}", index),
                MirrorMemoryConfig {
                    readable: false,
                    writable: true,
                    source_addresses,
                    source_address_space: cpu_address_space,
                    destination_addresses: 0x0000..=0x003f,
                    destination_address_space: cpu_address_space,
                },
            );

            machine = machine_builder;
            mirror_component_ids.push(component.id());
        }

        for (index, source_addresses) in tia_read_register_mirror_ranges().enumerate() {
            let (machine_builder, component) = machine.insert_component(
                &format!("tia_read_mirror_{}", index),
                MirrorMemoryConfig {
                    readable: true,
                    writable: false,
                    source_addresses,
                    source_address_space: cpu_address_space,
                    destination_addresses: 0x0000..=0x000f,
                    destination_address_space: cpu_address_space,
                },
            );

            machine = machine_builder;
            mirror_component_ids.push(component.id());
        }

        for (index, source_addresses) in riot_register_mirror_ranges().enumerate() {
            let (machine_builder, component) = machine.insert_component(
                &format!("mos6532_riot_register_mirror_{}", index),
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    source_addresses,
                    source_address_space: cpu_address_space,
                    destination_addresses: 0x280..=0x283,
                    destination_address_space: cpu_address_space,
                },
            );

            machine = machine_builder;
            mirror_component_ids.push(component.id());
        }

        for (index, source_addresses) in riot_ram_mirror_ranges().enumerate() {
            machine = machine
                .insert_component(
                    &format!("mos6532_riot_ram_mirror_{}", index),
                    MirrorMemoryConfig {
                        readable: true,
                        writable: true,
                        source_addresses,
                        source_address_space: cpu_address_space,
                        destination_addresses: 0x80..=0xff,
                        destination_address_space: cpu_address_space,
                    },
                )
                .0;
        }

        let (machine, mos6532_riot) = match region {
            RegionSelection::Ntsc => common::<Ntsc, _>(cpu_address_space, machine, mirror_component_ids),
            RegionSelection::Pal => common::<Pal, _>(cpu_address_space, machine, mirror_component_ids),
            RegionSelection::Secam => common::<Secam, _>(cpu_address_space, machine, mirror_component_ids),
        };

        let (machine, _) =
            machine.insert_component("joystick", Atari2600JoystickConfig { mos6532_riot });

        machine
    }
}

fn common<R: Region, P: Platform<GraphicsApi: SupportedGraphicsApiTia>>(
    cpu_address_space: AddressSpaceHandle,
    machine: MachineBuilder<P>,
    mirror_component_ids: impl IntoIterator<Item = ComponentId>,
) -> (MachineBuilder<P>, ComponentRef<Mos6532Riot>) {
    let (machine, cpu) = machine.insert_component(
        "mos_6502",
        Mos6502Config {
            frequency: R::frequency() / Ratio::from_integer(3),
            kind: Mos6502Kind::Mos6507,
            assigned_address_space: cpu_address_space,
            broken_ror: false,
        },
    );

    let (machine, mos6532_riot) = machine.insert_component(
        "mos6532_riot",
        Mos6532RiotConfig {
            frequency: R::frequency() / Ratio::from_integer(3),
            registers_assigned_address: 0x280,
            assigned_address_space: cpu_address_space,
        },
    );

    // For the love of god do not shadow this
    let (machine, _) = machine.insert_component_with_dependencies(
        "mos6532_riot_ram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x80..=0xff,
            assigned_address_space: cpu_address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0x80..=0xff,
                StandardMemoryInitialContents::Random,
            )]),
        },
        mirror_component_ids,
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

fn tia_read_register_mirror_ranges() -> impl Iterator<Item = RangeInclusive<Address>> {
    (0x0000..=0x0fff).step_by(0x20).skip(1).filter_map(|base| {
        let reduced_base = base & 0xff;

        if [0x00, 0x20, 0x40, 0x60].contains(&reduced_base) {
            return Some(base..=base + 0x3f);
        }

        None
    })
}

fn tia_write_register_mirror_ranges() -> impl Iterator<Item = RangeInclusive<Address>> {
    (0x0000..=0x0fff).step_by(0x40).skip(1).filter_map(|base| {
        let reduced_base = base & 0xff;

        if [0x00, 0x40].contains(&reduced_base) {
            return Some(base..=base + 0x3f);
        }

        None
    })
}

// 0x280..=0x283
fn riot_register_mirror_ranges() -> impl Iterator<Item = RangeInclusive<Address>> {
    (1..16).map(|i| {
        let base = 0x280 + i * 0x08;

        base..=base + 0x03
    })
}

// 0x80..=0xff
fn riot_ram_mirror_ranges() -> impl Iterator<Item = RangeInclusive<Address>> {
    [0x0180, 0x0480, 0x0580, 0x0880, 0x0980, 0x0c80, 0x0d80]
        .into_iter()
        .map(|range| range..=range + 0x7f)
}
