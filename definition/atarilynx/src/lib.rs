use multiemu_config::Environment;
use multiemu_definition_misc::memory::rom::{RomMemory, RomMemoryConfig};
use multiemu_machine::{builder::MachineBuilder, display::shader::ShaderCache};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{AtariSystem, GameSystem},
};
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

pub fn manifest(
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: ShaderCache,
) -> MachineBuilder {
    let machine = MachineBuilder::new(
        GameSystem::Atari(AtariSystem::Lynx),
        rom_manager.clone(),
        environment,
        shader_cache,
    );

    let (machine, cpu_address_space) = machine.insert_address_space("cpu", 16);

    let (machine, _) = machine.insert_component::<RomMemory>(
        "bootstrap",
        RomMemoryConfig {
            // "[BIOS] Atari Lynx (World).lyx"
            rom: RomId::from_str("e4ed47fae31693e016b081c6bda48da5b70d7ccb").unwrap(),
            max_word_size: 1,
            assigned_range: 0xfe00..=0xffff,
            assigned_address_space: cpu_address_space,
        },
    );

    machine
}
