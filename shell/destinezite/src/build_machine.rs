use multiemu_base::{
    graphics::software::Software,
    platform::Platform,
    program::{AtariSystem, MachineId, NintendoSystem, OtherSystem},
};
use multiemu_definition_atari2600::Atari2600;
use multiemu_definition_atarilynx::AtariLynx;
use multiemu_definition_chip8::Chip8;
use multiemu_definition_nes::Nes;
use multiemu_frontend::MachineFactories;

#[cfg(feature = "vulkan")]
pub fn get_vulkan_factories<P: Platform<GraphicsApi = multiemu_base::graphics::vulkan::Vulkan>>()
-> MachineFactories<P> {
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
