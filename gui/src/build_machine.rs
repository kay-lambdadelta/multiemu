use multiemu_config::Environment;
use multiemu_machine::builder::MachineBuilder;
use multiemu_rom::system::NintendoSystem;
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
        GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => {
            multiemu_definition_nes::build_machine(user_specified_roms, rom_manager, environment)
        }
        GameSystem::Other(OtherSystem::Chip8) => {
            multiemu_definition_chip8::build_machine(user_specified_roms, rom_manager, environment)
        }
        _ => unimplemented!(),
    }
}
