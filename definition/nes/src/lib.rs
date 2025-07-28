pub use cartridge::ines::INes;
use cartridge::{NesCartridgeConfig, ines::TimingMode};
use multiemu_definition_misc::memory::{
    mirror::MirrorMemoryConfig,
    standard::{StandardMemoryConfig, StandardMemoryInitialContents},
};
use multiemu_definition_mos6502::{Mos6502Config, Mos6502Kind};
use multiemu_runtime::{MachineFactory, builder::MachineBuilder, platform::Platform};
use num::rational::Ratio;
use ppu::NesPpuConfig;
use rangemap::RangeInclusiveMap;

mod apu;
mod cartridge;
mod ppu;

#[derive(Debug, Default)]
pub struct Nes;

impl<P: Platform> MachineFactory<P> for Nes {
    fn construct(&self, machine: MachineBuilder<P>) -> MachineBuilder<P> {
        let (machine, cpu_address_space) = machine.insert_address_space(16);
        let (machine, ppu_address_space) = machine.insert_address_space(16);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0x0000..=0x07ff,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0x0000..=0xffff,
                    StandardMemoryInitialContents::Random,
                )]),
                sram: false,
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror-0",
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 0x0800..=0x0fff,
                source_address_space: cpu_address_space,
                destination_addresses: 0x0000..=0x07ff,
                destination_address_space: cpu_address_space,
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror-1",
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 0x1000..=0x17ff,
                source_address_space: cpu_address_space,
                destination_addresses: 0x0800..=0x0fff,
                destination_address_space: cpu_address_space,
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror-2",
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 0x1800..=0x1fff,
                source_address_space: cpu_address_space,
                destination_addresses: 0x1000..=0x17ff,
                destination_address_space: cpu_address_space,
            },
        );

        let (mut machine, _) = machine.insert_default_component::<NesPpuConfig>("ppu");

        for (index, address) in (0x2000..=0x3fff).step_by(8).skip(1).enumerate() {
            machine = machine
                .insert_component(
                    &format!("ppu-register-mirror-{}", index),
                    MirrorMemoryConfig {
                        readable: true,
                        writable: true,
                        source_addresses: address..=address + 7,
                        source_address_space: cpu_address_space,
                        destination_addresses: address - 8..=address - 1,
                        destination_address_space: cpu_address_space,
                    },
                )
                .0;
        }

        let rom = machine.user_specified_roms().unwrap().main.id;
        let (machine, cartridge) = machine.insert_component(
            "cartridge",
            NesCartridgeConfig {
                rom,
                cpu_address_space,
                ppu_address_space,
            },
        );

        // Grab the timing mode
        let timing_mode = cartridge.interact(|cart| cart.rom().timing_mode).unwrap();

        let processor_frequency = Ratio::from_integer(match timing_mode {
            TimingMode::Ntsc => 1789773,
            TimingMode::Pal => 2097152,
            TimingMode::Multi => 1789773,
            TimingMode::Dendy => 1773448,
        });
        let (machine, _) = machine.insert_component(
            "processor",
            Mos6502Config {
                frequency: processor_frequency,
                assigned_address_space: cpu_address_space,
                kind: Mos6502Kind::Ricoh2A0x,
                broken_ror: false,
            },
        );

        machine
    }
}
