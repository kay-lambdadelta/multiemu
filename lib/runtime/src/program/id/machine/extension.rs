use super::{AtariSystem, MachineId, NintendoSystem, OtherSystem, SegaSystem, SonySystem};

// TODO: This should factor in rom format to handle the more tricky formats

/// Get a well known file extension for the files this system supports
pub fn get_extension(system: MachineId) -> Option<&'static str> {
    Some(match system {
        MachineId::Nintendo(NintendoSystem::GameBoy) => "gb",
        MachineId::Nintendo(NintendoSystem::GameBoyColor) => "gbc",
        MachineId::Nintendo(NintendoSystem::GameBoyAdvance) => "gba",
        MachineId::Nintendo(NintendoSystem::GameCube) => "iso",
        MachineId::Nintendo(NintendoSystem::Wii) => "iso",
        MachineId::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => "nes",
        MachineId::Nintendo(NintendoSystem::SuperNintendoEntertainmentSystem) => "sfc",
        MachineId::Nintendo(NintendoSystem::Nintendo64) => "z64",
        MachineId::Sega(SegaSystem::GameGear) => "gg",
        MachineId::Sega(SegaSystem::MasterSystem) => "sms",
        MachineId::Sega(SegaSystem::Genesis) => "md",
        MachineId::Sega(SegaSystem::Sega32X) => "32x",
        MachineId::Sega(SegaSystem::SegaCD) => "iso",
        MachineId::Sony(SonySystem::PlaystationPortable) => "iso",
        MachineId::Atari(AtariSystem::Atari2600) => "a26",
        MachineId::Atari(AtariSystem::Atari5200) => "a52",
        MachineId::Atari(AtariSystem::Atari7800) => "a78",
        MachineId::Atari(AtariSystem::Lynx) => "lnx",
        MachineId::Atari(AtariSystem::Jaguar) => "jag",
        MachineId::Other(OtherSystem::Chip8) => "ch8",
        _ => return None,
    })
}
