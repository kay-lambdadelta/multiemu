use std::sync::{Arc, RwLock};

pub use cartridge::ines::INes;
use cartridge::{NesCartridge, NesCartridgeConfig, ines::TimingMode};
use multiemu_config::Environment;
use multiemu_definition_m6502::{M6502, M6502Config, M6502Kind};
use multiemu_definition_misc::memory::{
    mirror::{MirrorMemory, MirrorMemoryConfig, PermissionSpace},
    standard::{StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents},
};
use multiemu_machine::{builder::MachineBuilder, display::shader::ShaderCache};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{GameSystem, NintendoSystem},
};
use num::rational::Ratio;
use ppu::NesPpu;

mod apu;
mod cartridge;
mod ppu;

pub fn manifest(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: Arc<ShaderCache>,
) -> MachineBuilder {
    let machine = multiemu_machine::builder::MachineBuilder::new(
        GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem),
        rom_manager.clone(),
        environment.clone(),
        shader_cache.clone(),
    );

    let (cpu_address_space, machine) = machine.insert_address_space("cpu", 16);
    let (ppu_address_space, machine) = machine.insert_address_space("ppu", 16);

    let machine = machine
        .insert_component::<NesCartridge>(
            "cartridge",
            NesCartridgeConfig {
                rom: user_specified_roms[0],
                cpu_address_space,
                ppu_address_space,
            },
        )
        .insert_component::<StandardMemory>(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                max_word_size: 2,
                assigned_range: 0x0000..=0x07ff,
                assigned_address_space: cpu_address_space,
                initial_contents: vec![StandardMemoryInitialContents::Random],
            },
        )
        .insert_component::<MirrorMemory>(
            "workram-mirror",
            MirrorMemoryConfig::default().insert_range(
                0x0800..=0x0fff,
                cpu_address_space,
                0x0000..=0x07ff,
                cpu_address_space,
                [PermissionSpace::Read, PermissionSpace::Write],
            ),
        )
        .insert_default_component::<NesPpu>("ppu");

    // Grab the timing mode
    let mut timing_mode = TimingMode::Ntsc;
    machine
        .component_store()
        .interact_by_name_local::<NesCartridge>("cartridge", |cart| {
            timing_mode = cart.rom().timing_mode
        })
        .unwrap();

    let processor_frequency = Ratio::from_integer(match timing_mode {
        TimingMode::Ntsc => 1789773,
        TimingMode::Pal => 2097152,
        TimingMode::Multi => 1789773,
        TimingMode::Dendy => 1773448,
    });

    machine.insert_component::<M6502>(
        "processor",
        M6502Config {
            frequency: processor_frequency,
            assigned_address_space: cpu_address_space,
            kind: M6502Kind::R2A0x,
        },
    )
}
