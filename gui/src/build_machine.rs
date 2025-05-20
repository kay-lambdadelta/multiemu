use multiemu_config::Environment;
use multiemu_definition_atari2600::Atari2600;
use multiemu_definition_atarilynx::AtariLynx;
use multiemu_definition_chip8::Chip8;
use multiemu_definition_nes::Nes;
use multiemu_machine::{
    MachineFactory,
    builder::MachineBuilder,
    display::{backend::RenderApi, shader::ShaderCache},
};
use multiemu_rom::{
    id::RomId,
    manager::RomManager,
    system::{AtariSystem, GameSystem, NintendoSystem, OtherSystem},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct MachineFactories<R: RenderApi>(HashMap<GameSystem, Box<dyn MachineFactory<R>>>);

impl<R: RenderApi> MachineFactories<R> {
    pub fn insert_factory<M: MachineFactory<R> + Default>(&mut self, system: GameSystem) {
        self.0.insert(system, Box::new(M::default()));
    }

    pub fn construct_machine(
        &self,
        system: GameSystem,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
    ) -> MachineBuilder<R> {
        self.0
            .get(&system)
            .unwrap_or_else(|| panic!("No factory for system {:?}", system))
            .construct(user_specified_roms, rom_manager, environment, shader_cache)
    }
}

impl<R: RenderApi> Default for MachineFactories<R> {
    fn default() -> Self {
        Self(Default::default())
    }
}

// These functions are a bit of a hack to be able to show rust we only care about concrete types for render api
// Also future compatible for when we add more machines that dont cover all backends

pub fn get_software_factories()
-> MachineFactories<multiemu_machine::display::backend::software::SoftwareRendering> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(GameSystem::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(GameSystem::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(GameSystem::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(GameSystem::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}

#[cfg(all(feature = "vulkan", platform_desktop))]
pub fn get_vulkan_factories()
-> MachineFactories<multiemu_machine::display::backend::vulkan::VulkanRendering> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(GameSystem::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(GameSystem::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(GameSystem::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(GameSystem::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}
