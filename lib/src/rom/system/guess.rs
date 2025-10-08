use super::{AtariSystem, NintendoSystem, OtherSystem, SegaSystem, System};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::LazyLock,
};

struct MagicTableEntry {
    bytes: &'static [u8],
    offset: usize,
}

/// Magic number table
static MAGIC_TABLE: LazyLock<HashMap<System, Vec<MagicTableEntry>>> = LazyLock::new(|| {
    let mut table: HashMap<System, Vec<MagicTableEntry>> = HashMap::new();

    table
        .entry(System::Nintendo(NintendoSystem::GameBoy))
        .or_default()
        .extend([MagicTableEntry {
            bytes: &[0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b],
            offset: 0x134,
        }]);

    table
        .entry(System::Nintendo(
            NintendoSystem::NintendoEntertainmentSystem,
        ))
        .or_default()
        .extend([MagicTableEntry {
            bytes: b"NES\x1a",
            offset: 0x00,
        }]);

    table
        .entry(System::Sega(SegaSystem::Genesis))
        .or_default()
        .extend([
            MagicTableEntry {
                bytes: b"SEGA GENESIS",
                offset: 0x100,
            },
            MagicTableEntry {
                bytes: b"SEGA MEGA DRIVE",
                offset: 0x100,
            },
        ]);

    table
        .entry(System::Sega(SegaSystem::MasterSystem))
        .or_default()
        .extend([
            MagicTableEntry {
                bytes: b"TMR SEGA",
                offset: 0x1ff0,
            },
            MagicTableEntry {
                bytes: b"TMR SEGA",
                offset: 0x3ff0,
            },
            MagicTableEntry {
                bytes: b"TMR SEGA",
                offset: 0x7ff0,
            },
        ]);

    table
});

/// Guess a the system from a rom file on disk, using a variety of heuristics
pub fn guess_system(rom_path: impl AsRef<Path>) -> Option<System> {
    let rom_path = rom_path.as_ref();
    let mut rom = File::open(rom_path).unwrap();

    // This goes first since a lot of roms have misleading or nonexistent magic bytes
    if let Some(system) = guess_by_extension(rom_path) {
        return Some(system);
    }

    let mut read_buffer = Vec::new();
    for (system, entry) in MAGIC_TABLE
        .iter()
        .flat_map(|(system, entries)| entries.iter().map(|entry| (*system, entry)))
    {
        read_buffer.resize(entry.bytes.len(), 0);

        if rom.seek(SeekFrom::Start(entry.offset as u64)).is_err() {
            continue;
        }

        if rom.read_exact(&mut read_buffer).is_err() {
            continue;
        }

        if read_buffer == entry.bytes {
            tracing::info!(
                "Guessed system of ROM at {} from its magic as {}",
                rom_path.display(),
                system
            );

            return Some(system);
        }
    }

    None
}

/// Try to guess the system from the file extension
fn guess_by_extension(rom: &Path) -> Option<System> {
    if let Some(file_extension) = rom
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        && let Some(system) = match file_extension.as_str() {
            "gb" => Some(System::Nintendo(NintendoSystem::GameBoy)),
            "gbc" => Some(System::Nintendo(NintendoSystem::GameBoyColor)),
            "gba" => Some(System::Nintendo(NintendoSystem::GameBoyAdvance)),
            "nes" => Some(System::Nintendo(
                NintendoSystem::NintendoEntertainmentSystem,
            )),
            "sfc" | "smc" => Some(System::Nintendo(
                NintendoSystem::SuperNintendoEntertainmentSystem,
            )),
            "n64" | "z64" => Some(System::Nintendo(NintendoSystem::Nintendo64)),
            "md" => Some(System::Sega(SegaSystem::MasterSystem)),
            "gg" => Some(System::Sega(SegaSystem::GameGear)),
            "ch8" | "c8" => Some(System::Other(OtherSystem::Chip8)),
            "a26" => Some(System::Atari(AtariSystem::Atari2600)),
            "a52" => Some(System::Atari(AtariSystem::Atari5200)),
            "a78" => Some(System::Atari(AtariSystem::Atari7800)),
            _ => None,
        }
    {
        tracing::info!(
            "Guessed system of ROM at {} from file extension {} as {}",
            rom.display(),
            file_extension,
            system
        );
        return Some(system);
    }

    None
}
