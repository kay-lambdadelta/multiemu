use super::{AtariSystem, NintendoSystem, OtherSystem, SegaSystem, SonySystem, System};

// TODO: This should factor in rom format to handle the more tricky formats

/// Get a well known file extension for the files this system supports
pub fn get_extension(system: System) -> Option<&'static str> {
    Some(match system {
        System::Nintendo(NintendoSystem::GameBoy) => "gb",
        System::Nintendo(NintendoSystem::GameBoyColor) => "gbc",
        System::Nintendo(NintendoSystem::GameBoyAdvance) => "gba",
        System::Nintendo(NintendoSystem::GameCube) => "iso",
        System::Nintendo(NintendoSystem::Wii) => "iso",
        System::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => "nes",
        System::Nintendo(NintendoSystem::SuperNintendoEntertainmentSystem) => "sfc",
        System::Nintendo(NintendoSystem::Nintendo64) => "z64",
        System::Sega(SegaSystem::GameGear) => "gg",
        System::Sega(SegaSystem::MasterSystem) => "sms",
        System::Sega(SegaSystem::Genesis) => "md",
        System::Sega(SegaSystem::Sega32X) => "32x",
        System::Sega(SegaSystem::SegaCD) => "iso",
        System::Sony(SonySystem::PlaystationPortable) => "iso",
        System::Atari(AtariSystem::Atari2600) => "a26",
        System::Atari(AtariSystem::Atari5200) => "a52",
        System::Atari(AtariSystem::Atari7800) => "a78",
        System::Atari(AtariSystem::Lynx) => "lnx",
        System::Atari(AtariSystem::Jaguar) => "jag",
        System::Other(OtherSystem::Chip8) => "ch8",
        _ => return None,
    })
}
