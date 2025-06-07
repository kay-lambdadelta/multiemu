use multiemu_definition_atari2600::Atari2600;
use multiemu_definition_atarilynx::AtariLynx;
use multiemu_definition_chip8::Chip8;
use multiemu_definition_nes::Nes;
use multiemu_graphics::{GraphicsApi, Software};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{AtariSystem, GameSystem, NintendoSystem, OtherSystem},
};
use multiemu_runtime::{MachineFactory, builder::MachineBuilder};
use num::rational::Ratio;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct MachineFactories<R: GraphicsApi>(HashMap<GameSystem, Box<dyn MachineFactory<R, f32>>>);

impl<R: GraphicsApi> MachineFactories<R> {
    pub fn insert_factory<M: MachineFactory<R, f32> + Default>(&mut self, system: GameSystem) {
        self.0.insert(system, Box::new(M::default()));
    }

    pub fn construct_machine(
        &self,
        system: GameSystem,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        sample_rate: Ratio<u32>,
    ) -> MachineBuilder<R> {
        self.0
            .get(&system)
            .unwrap_or_else(|| panic!("No factory for system {:?}", system))
            .construct(user_specified_roms, rom_manager, sample_rate)
    }
}

impl<R: GraphicsApi> Default for MachineFactories<R> {
    fn default() -> Self {
        Self(Default::default())
    }
}

// These functions are a bit of a hack to be able to show rust we only care about concrete types for render api
// Also future compatible for when we add more machines that don't cover all backends

pub fn get_software_factories() -> MachineFactories<Software> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(GameSystem::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(GameSystem::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(GameSystem::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(GameSystem::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}

#[cfg(feature = "vulkan")]
pub fn get_vulkan_factories() -> MachineFactories<multiemu_graphics::Vulkan> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(GameSystem::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(GameSystem::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(GameSystem::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(GameSystem::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}
