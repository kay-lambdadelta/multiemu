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
use multiemu_rom::ROM_INFORMATION_TABLE;
use multiemu_runtime::{
    MachineFactory,
    builder::MachineBuilder,
    component::{ComponentId, ComponentRef},
    memory::{Address, AddressSpaceHandle},
    platform::Platform,
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
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
        let rom = machine.user_specified_roms().unwrap().main;
        let rom_manager = machine.rom_manager().clone();

        // Atari 2600 CPU only has 13 address lines
        let (machine, cpu_address_space) = machine.insert_address_space(13);

        // Extract information on the rom loaded
        let database_transaction = rom_manager.rom_information.begin_read().unwrap();
        let table = database_transaction
            .open_multimap_table(ROM_INFORMATION_TABLE)
            .unwrap();
        let rom_info = table.get(&rom).unwrap().next().unwrap().unwrap().value();

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
                rom,
                cpu_address_space,
                force_cart_type: None,
            },
        );

        for (index, source_addresses) in tia_register_mirror_ranges().enumerate() {
            let (machine_builder, component) = machine.insert_component(
                &format!("tia_mirror_{}", index),
                MirrorMemoryConfig {
                    readable: true,
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

        for (index, source_addresses) in riot_register_mirror_ranges().enumerate() {
            let (machine_builder, component) = machine.insert_component(
                &format!("mos6532_riot_register_mirror_{}", index),
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    source_addresses,
                    source_address_space: cpu_address_space,
                    destination_addresses: 0x280..=0x29f,
                    destination_address_space: cpu_address_space,
                },
            );

            machine = machine_builder;
            mirror_component_ids.push(component.id());
        }

        for (index, source_addresses) in riot_ram_mirror_ranges().enumerate() {
            let (machine_builder, component) = machine.insert_component(
                &format!("mos6532_riot_ram_mirror_{}", index),
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    source_addresses,
                    source_address_space: cpu_address_space,
                    destination_addresses: 0x80..=0xff,
                    destination_address_space: cpu_address_space,
                },
            );

            machine = machine_builder;
            mirror_component_ids.push(component.id());
        }

        let (machine, mos6532_riot) = match region {
            RegionSelection::Ntsc => {
                common::<Ntsc, _>(cpu_address_space, machine, mirror_component_ids)
            }
            RegionSelection::Pal => {
                common::<Pal, _>(cpu_address_space, machine, mirror_component_ids)
            }
            RegionSelection::Secam => {
                common::<Secam, _>(cpu_address_space, machine, mirror_component_ids)
            }
        };

        let (machine, _) =
            machine.insert_component("joystick", Atari2600JoystickConfig { mos6532_riot });

        machine
    }
}

fn common<R: Region, P: Platform<GraphicsApi: SupportedGraphicsApiTia>>(
    cpu_address_space: AddressSpaceHandle,
    machine: MachineBuilder<P>,
    mirror_component_ids: Vec<ComponentId>,
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

    let (machine, mos6532_riot) = machine.insert_component_with_dependencies(
        "mos6532_riot",
        Mos6532RiotConfig {
            frequency: R::frequency() / Ratio::from_integer(3),
            registers_assigned_address: 0x280,
            assigned_address_space: cpu_address_space,
        },
        mirror_component_ids.clone(),
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
            sram: false,
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
        0x2280, 0x22a0, 0x22c0, 0x22e0,
        0x2380, 0x23a0, 0x23c0, 0x23e0,
        0x2680, 0x26a0, 0x26c0, 0x26e0,
        0x2780, 0x27a0, 0x27c0, 0x27e0,
        0x2a80, 0x2aa0, 0x2ac0, 0x2ae0,
        0x2b80, 0x2ba0, 0x2bc0, 0x2be0,
        0x2e80, 0x2ea0, 0x2ec0, 0x2ee0,
        0x2f80, 0x2fa0, 0x2fc0, 0x2fe0
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

#[cfg(test)]
mod tests {
    use crate::Atari2600;
    use multiemu_config::{ENVIRONMENT_LOCATION, Environment};
    use multiemu_rom::{AtariSystem, RomId, RomManager, System};
    use multiemu_runtime::{
        MachineFactory, UserSpecifiedRoms,
        builder::MachineBuilder,
        platform::TestPlatform,
        utils::{DirectMainThreadExecutor, set_main_thread},
    };
    use multiemu_save::{SaveManager, SnapshotManager};
    use num::rational::Ratio;
    use std::{borrow::Cow, fs::File, ops::Deref, str::FromStr, sync::Arc};

    #[test]
    fn riot_ram_access() {
        set_main_thread();

        let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
        let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

        let rom_manager = Arc::new(
            RomManager::new(
                Some(environment.database_location.0.clone()),
                Some(environment.rom_store_directory.0.clone()),
            )
            .unwrap(),
        );
        let save_manager = Arc::new(SaveManager::new(None));
        let snapshot_manager = Arc::new(SnapshotManager::new(None));

        let machine = MachineBuilder::new(
            Some(UserSpecifiedRoms {
                // Donkey Kong (USA).a26
                main: RomId::from_str("6e6e37ec8d66aea1c13ed444863e3db91497aa35").unwrap(),
                sub: Cow::Borrowed(&[]),
            }),
            System::Atari(AtariSystem::Atari2600),
            rom_manager,
            save_manager,
            snapshot_manager,
            Ratio::from_integer(44100),
            Arc::new(DirectMainThreadExecutor),
        );

        let machine = MachineFactory::<TestPlatform>::construct(&Atari2600, machine)
            .build(Default::default());

        let cpu_address_space = machine.memory_access_table.address_spaces().next().unwrap();

        let _: u8 = machine
            .memory_access_table
            .read_le_value(0x180, cpu_address_space)
            .expect(&format!("{:#04x?}", machine));
    }
}
