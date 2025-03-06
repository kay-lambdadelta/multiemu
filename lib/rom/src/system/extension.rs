use crate::system::{AtariSystem, GameSystem, NintendoSystem, OtherSystem, SegaSystem, SonySystem};

// TODO: This should factor in rom format to handle the more tricky formats

/// Get a well known file extension for the files this system supports
pub fn get_extension(system: GameSystem) -> Option<&'static str> {
    Some(match system {
        GameSystem::Nintendo(NintendoSystem::GameBoy) => "gb",
        GameSystem::Nintendo(NintendoSystem::GameBoyColor) => "gbc",
        GameSystem::Nintendo(NintendoSystem::GameBoyAdvance) => "gba",
        GameSystem::Nintendo(NintendoSystem::GameCube) => "iso",
        GameSystem::Nintendo(NintendoSystem::Wii) => "iso",
        GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => "nes",
        GameSystem::Nintendo(NintendoSystem::SuperNintendoEntertainmentSystem) => "sfc",
        GameSystem::Nintendo(NintendoSystem::Nintendo64) => "z64",
        GameSystem::Sega(SegaSystem::GameGear) => "gg",
        GameSystem::Sega(SegaSystem::MasterSystem) => "sms",
        GameSystem::Sega(SegaSystem::Genesis) => "md",
        GameSystem::Sega(SegaSystem::Sega32X) => "32x",
        GameSystem::Sega(SegaSystem::SegaCD) => "iso",
        GameSystem::Sony(SonySystem::PlaystationPortable) => "iso",
        GameSystem::Atari(AtariSystem::Atari2600) => "a26",
        GameSystem::Atari(AtariSystem::Atari5200) => "a52",
        GameSystem::Atari(AtariSystem::Atari7800) => "a78",
        GameSystem::Atari(AtariSystem::Lynx) => "lnx",
        GameSystem::Atari(AtariSystem::Jaguar) => "jag",
        GameSystem::Other(OtherSystem::Chip8) => "ch8",
        _ => return None,
    })
}
