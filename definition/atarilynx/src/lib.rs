use mapctl::MapctlConfig;
use multiemu_definition_misc::memory::{
    null::NullMemoryConfig,
    rom::RomMemoryConfig,
    standard::{StandardMemoryConfig, StandardMemoryInitialContents},
};
use multiemu_rom::RomId;
use multiemu_runtime::{
    MachineFactory, builder::MachineBuilder, memory::Address, platform::Platform,
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use std::{ops::RangeInclusive, str::FromStr};

use crate::suzy::SuzyConfig;

mod mapctl;
mod mikey;
mod suzy;

const SUZY_ADDRESSES: RangeInclusive<Address> = 0xfc00..=0xfcff;
const MIKEY_ADDRESSES: RangeInclusive<Address> = 0xfd00..=0xfdff;
const VECTOR_ADDRESSES: RangeInclusive<Address> = 0xfff8..=0xffff;
const RESERVED_MEMORY_ADDRESS: Address = 0xfff8;
const MAPCTL_ADDRESS: Address = 0xfff9;

#[derive(Debug, Default)]
pub struct AtariLynx;

impl<P: Platform> MachineFactory<P> for AtariLynx {
    fn construct(&self, machine: MachineBuilder<P>) -> MachineBuilder<P> {
        // 16 Mhz
        let base_clock = Ratio::from_integer(16000000);
        let (machine, cpu_address_space) = machine.insert_address_space(16);

        // A good portion of this will be initially shadowed
        let (machine, ram) = machine.insert_component(
            "ram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0x0000..=0xffff,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0x0000..=0xffff,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
                sram: false,
            },
        );

        let (machine, reserved) = machine.insert_component(
            "reserved",
            NullMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: RESERVED_MEMORY_ADDRESS..=RESERVED_MEMORY_ADDRESS,
                assigned_address_space: cpu_address_space,
            },
        );

        let (machine, rom) = machine.insert_component(
            "bootstrap",
            RomMemoryConfig {
                // "[BIOS] Atari Lynx (World).lyx"
                rom: RomId::from_str("e4ed47fae31693e016b081c6bda48da5b70d7ccb").unwrap(),
                assigned_range: 0xfe00..=0xffff,
                assigned_address_space: cpu_address_space,
                rom_range: 0x0000..=0x1fff,
            },
        );

        let (machine, suzy) = machine.insert_component("suzy", SuzyConfig { cpu_address_space });

        let (machine, _) = machine.insert_component(
            "mapctl",
            MapctlConfig {
                cpu_address_space,
                ram: ram.id(),
                suzy: suzy.id(),
                reserved: reserved.id(),
                mikey: todo!(),
                vector: todo!(),
            },
        );

        machine
    }
}
