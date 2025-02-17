use multiemu_config::Environment;
use multiemu_machine::builder::MachineBuilder;
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
        GameSystem::Nintendo(system) => {
            todo!()
        }
        GameSystem::Sega(sega_system) => todo!(),
        GameSystem::Sony(sony_system) => todo!(),
        GameSystem::Atari(atari_system) => todo!(),
        GameSystem::Other(OtherSystem::Chip8) => {
            multiemu_definition_chip8::build_machine(user_specified_roms, rom_manager, environment)
        }
        GameSystem::Unknown => todo!(),
    }
}
