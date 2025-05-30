pub use cartridge::ines::INes;
use cartridge::{NesCartridgeConfig, ines::TimingMode};
use multiemu_config::Environment;
use multiemu_definition_misc::memory::{
    mirror::MirrorMemoryConfig,
    standard::{StandardMemoryConfig, StandardMemoryInitialContents},
};
use multiemu_definition_mos6502::{Mos6502Config, Mos6502Kind};
use multiemu_machine::{
    MachineFactory,
    builder::MachineBuilder,
    display::{backend::RenderApi, shader::ShaderCache},
};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{GameSystem, NintendoSystem},
};
use num::rational::Ratio;
use ppu::NesPpuConfig;
use rangemap::RangeInclusiveMap;
use std::sync::{Arc, RwLock};

mod apu;
mod cartridge;
mod ppu;

#[derive(Debug, Default)]
pub struct Nes;

impl<R: RenderApi> MachineFactory<R> for Nes {
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
    ) -> MachineBuilder<R> {
        let machine = multiemu_machine::builder::MachineBuilder::new(
            GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem),
            rom_manager.clone(),
            environment.clone(),
            shader_cache.clone(),
        );

        let (machine, cpu_address_space) = machine.insert_address_space(16);
        let (machine, ppu_address_space) = machine.insert_address_space(16);

        let (machine, cartridge) = machine.insert_component(
            "cartridge",
            NesCartridgeConfig {
                rom: user_specified_roms[0],
                cpu_address_space,
                ppu_address_space,
            },
        );
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
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror",
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 0x0800..=0x0fff,
                source_address_space: cpu_address_space,
                destination_addresses: 0x0000..=0x07ff,
                destination_address_space: cpu_address_space,
            },
        );
        let (machine, _) = machine.insert_default_component::<NesPpuConfig>("ppu");

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
