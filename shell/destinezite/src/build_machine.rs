use fluxemu_definition_atari2600::Atari2600;
use fluxemu_definition_atarilynx::AtariLynx;
use fluxemu_definition_chip8::Chip8;
use fluxemu_definition_nes::Nes;
use fluxemu_frontend::MachineFactories;
use fluxemu_runtime::{
    graphics::software::Software,
    platform::Platform,
    program::{AtariSystem, MachineId, NintendoSystem, OtherSystem},
};

#[cfg(feature = "vulkan")]
pub fn get_vulkan_factories<
    P: Platform<GraphicsApi = fluxemu_runtime::graphics::vulkan::Vulkan>,
>() -> MachineFactories<P> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(MachineId::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(MachineId::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(MachineId::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(MachineId::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}

pub fn get_software_factories<P: Platform<GraphicsApi = Software>>() -> MachineFactories<P> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(MachineId::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(MachineId::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(MachineId::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(MachineId::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}
