use multiemu_audio::FromSample;
use multiemu_definition_atari2600::Atari2600;
use multiemu_definition_atarilynx::AtariLynx;
use multiemu_definition_chip8::Chip8;
use multiemu_definition_nes::Nes;
use multiemu_frontend::MachineFactories;
use multiemu_rom::{AtariSystem, GameSystem, NintendoSystem, OtherSystem};
use multiemu_runtime::platform::Platform;

#[cfg(feature = "vulkan")]
pub fn get_vulkan_factories<
    P: Platform<SampleFormat: FromSample<f32>, GraphicsApi = multiemu_graphics::vulkan::Vulkan>,
>() -> MachineFactories<P> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(GameSystem::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(GameSystem::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(GameSystem::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(GameSystem::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}

pub fn get_software_factories<
    P: Platform<SampleFormat: FromSample<f32>, GraphicsApi = multiemu_graphics::software::Software>,
>() -> MachineFactories<P> {
    let mut factories = MachineFactories::default();

    factories.insert_factory::<Atari2600>(GameSystem::Atari(AtariSystem::Atari2600));
    factories.insert_factory::<AtariLynx>(GameSystem::Atari(AtariSystem::Lynx));
    factories.insert_factory::<Chip8>(GameSystem::Other(OtherSystem::Chip8));
    factories.insert_factory::<Nes>(GameSystem::Nintendo(
        NintendoSystem::NintendoEntertainmentSystem,
    ));

    factories
}
