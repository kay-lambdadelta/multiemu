use multiemu_definition_atari2600::Atari2600;
use multiemu_definition_atarilynx::AtariLynx;
use multiemu_definition_chip8::Chip8;
use multiemu_definition_nes::Nes;
use multiemu_frontend::MachineFactories;
use multiemu_rom::{AtariSystem, NintendoSystem, OtherSystem, System};
use multiemu_runtime::platform::Platform;

#[cfg(feature = "vulkan")]
pub fn get_vulkan_factories<P: Platform<GraphicsApi = multiemu_graphics::vulkan::Vulkan>>()
-> MachineFactories<P> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(System::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(System::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(System::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(System::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}

pub fn get_software_factories<P: Platform<GraphicsApi = multiemu_graphics::software::Software>>()
-> MachineFactories<P> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(System::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(System::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(System::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(System::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}
