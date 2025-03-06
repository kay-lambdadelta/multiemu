use multiemu_config::Environment;
use multiemu_machine::builder::MachineBuilder;
use multiemu_rom::system::{AtariSystem, NintendoSystem};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{GameSystem, OtherSystem},
};
use std::sync::{Arc, RwLock};

pub fn build_machine(
    game_system: GameSystem,
    user_specified_roms: Vec<RomId>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
) -> MachineBuilder {
    match game_system {
        GameSystem::Atari(AtariSystem::Atari2600) => {
            multiemu_definition_atari2600::manifest(user_specified_roms, rom_manager, environment)
        }
        GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => {
            multiemu_definition_nes::manifest(user_specified_roms, rom_manager, environment)
        }
        GameSystem::Other(OtherSystem::Chip8) => {
            multiemu_definition_chip8::manifest(user_specified_roms, rom_manager, environment)
        }
        _ => unimplemented!(),
    }
}
