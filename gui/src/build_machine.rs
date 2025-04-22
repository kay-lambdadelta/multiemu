use multiemu_config::Environment;
use multiemu_machine::{builder::MachineBuilder, display::shader::ShaderCache};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{AtariSystem, GameSystem, NintendoSystem, OtherSystem},
};
use std::sync::{Arc, RwLock};

/// Build a machine from a game system
pub fn build_machine(
    game_system: GameSystem,
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: Arc<ShaderCache>,
) -> MachineBuilder {
    match game_system {
        GameSystem::Atari(AtariSystem::Atari2600) => multiemu_definition_atari2600::manifest(
            user_specified_roms,
            rom_manager,
            environment,
            shader_cache,
        ),
        GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => {
            multiemu_definition_nes::manifest(
                user_specified_roms,
                rom_manager,
                environment,
                shader_cache,
            )
        }
        GameSystem::Other(OtherSystem::Chip8) => multiemu_definition_chip8::manifest(
            user_specified_roms,
            rom_manager,
            environment,
            shader_cache,
        ),
        _ => unimplemented!("{:?}", game_system),
    }
}
