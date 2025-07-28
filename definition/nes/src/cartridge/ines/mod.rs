use bitvec::{field::BitField, prelude::Msb0, ptr::BitSpanError, view::BitView};
use expansion_device::DefaultExpansionDevice;
use rustc_hash::FxBuildHasher;
use std::{collections::HashMap, fmt::Debug, io::Read};
use thiserror::Error;

pub mod expansion_device;

pub const PRG_BANK_SIZE: usize = 16 * 1024;
pub const CHR_BANK_SIZE: usize = 8 * 1024;

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("Bitvec error: {0:#?}")]
    BitvecError(#[from] BitSpanError<u8>),
    #[error("Bad magic {bytes:?}")]
    BadMagic { bytes: [u8; 4] },
    #[error("Bad version {version}")]
    BadVersion { version: u8 },
    #[error("Bad console type")]
    BadConsoleType,
    #[error("Non volatile memory settings do not agree")]
    DisagreeingNonVolatileMemory,
    #[error("Not enough bytes left to be valid")]
    EarlyEOF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TimingMode {
    Ntsc,
    Pal,
    Multi,
    Dendy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConsoleType {
    NintendoEntertainmentSystem,
    NintendoVsSystem,
    NintendoPlaychoice10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum INesVersion {
    V1,
    V2 {
        console_type: ConsoleType,
        submapper: u8,
        misc_rom_count: u8,
        default_expansion_device: Option<DefaultExpansionDevice>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mirroring {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RomType {
    Trainer,
    Prg,
    Chr,
}

#[derive(Clone)]
pub struct INes {
    pub mapper: u16,
    pub alternative_nametables: bool,
    pub non_volatile_memory: bool,
    pub mirroring: Mirroring,
    pub version: INesVersion,
    pub timing_mode: TimingMode,
    pub roms: HashMap<RomType, Vec<u8>, FxBuildHasher>,
}

impl Debug for INes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("INes")
            .field("mapper", &self.mapper)
            .field("alternative_nametables", &self.alternative_nametables)
            .field("non_volatile_memory", &self.non_volatile_memory)
            .field("mirroring", &self.mirroring)
            .field("version", &self.version)
            .field("timing_mode", &self.timing_mode)
            // Omit the rom bytecode
            .field("roms", &())
            .finish()
    }
}

impl INes {
    pub fn parse(bytes: &[u8]) -> Result<Self, ParsingError> {
        if &bytes[0..4] != b"NES\x1a" {
            return Err(ParsingError::BadMagic {
                bytes: bytes[0..4].try_into().unwrap(),
            });
        }
        let bytes = &bytes[4..];

        let mut remaining = bytes.try_view_bits::<Msb0>()?;

        let mut prg_bank_count = remaining
            .get(0..8)
            .ok_or(ParsingError::EarlyEOF)?
            .load::<u16>();
        remaining = &remaining[8..];

        let mut chr_bank_count = remaining
            .get(0..8)
            .ok_or(ParsingError::EarlyEOF)?
            .load::<u16>();
        remaining = &remaining[8..];

        let mut mapper = remaining
            .get(0..4)
            .ok_or(ParsingError::EarlyEOF)?
            .load::<u16>();
        remaining = &remaining[4..];

        let alternative_nametables = *remaining.get(0).ok_or(ParsingError::EarlyEOF)?;
        remaining = &remaining[1..];

        let trainer = *remaining.get(0).ok_or(ParsingError::EarlyEOF)?;
        remaining = &remaining[1..];

        let non_volatile_memory = *remaining.get(0).ok_or(ParsingError::EarlyEOF)?;
        remaining = &remaining[1..];

        let mirroring = if *remaining.get(0).ok_or(ParsingError::EarlyEOF)? {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };
        remaining = &remaining[1..];

        mapper |= remaining
            .get(0..4)
            .ok_or(ParsingError::EarlyEOF)?
            .load::<u16>()
            << 4;
        remaining = &remaining[4..];

        // Get INES version
        let version = remaining
            .get(0..2)
            .ok_or(ParsingError::EarlyEOF)?
            .load::<u8>();
        let (version, timing_mode) = match version {
            0b00 => {
                remaining = &remaining[2..];

                (INesVersion::V1, TimingMode::Ntsc)
            }
            0b10 => {
                remaining = &remaining[2..];

                let console_type = match remaining
                    .get(0..2)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>()
                {
                    0b00 => Some(ConsoleType::NintendoEntertainmentSystem),
                    0b01 => Some(ConsoleType::NintendoVsSystem),
                    0b10 => Some(ConsoleType::NintendoPlaychoice10),
                    0b11 => None,
                    _ => unreachable!(),
                };
                remaining = &remaining[2..];

                let submapper = remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[4..];

                mapper |= remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u16>()
                    << 8;
                remaining = &remaining[4..];

                prg_bank_count |= remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u16>()
                    << 8;
                remaining = &remaining[4..];

                chr_bank_count |= remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u16>()
                    << 8;
                remaining = &remaining[4..];

                let prg_nvram_shift_count = remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[4..];

                let prg_ram_shift_count = remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[4..];

                if !non_volatile_memory && (prg_nvram_shift_count != 0 || prg_ram_shift_count != 0)
                {
                    return Err(ParsingError::DisagreeingNonVolatileMemory);
                }

                let chr_nvram_shift_count = remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[4..];

                let chr_ram_shift_count = remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[4..];

                // Skip unused bits
                remaining = &remaining[6..];

                let timing_mode = match remaining
                    .get(0..2)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>()
                {
                    0b00 => TimingMode::Ntsc,
                    0b01 => TimingMode::Pal,
                    0b10 => TimingMode::Multi,
                    0b11 => TimingMode::Dendy,
                    _ => unreachable!(),
                };
                remaining = &remaining[2..];

                let vs_system_type = remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[4..];

                let vs_ppu_type = remaining
                    .get(0..4)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[4..];

                // Skip unused bits
                remaining = &remaining[6..];

                let misc_rom_count = remaining
                    .get(0..2)
                    .ok_or(ParsingError::EarlyEOF)?
                    .load::<u8>();
                remaining = &remaining[2..];

                // Skip unused bits
                remaining = &remaining[2..];

                let default_expansion_device = DefaultExpansionDevice::new(
                    remaining
                        .get(0..6)
                        .ok_or(ParsingError::EarlyEOF)?
                        .load::<u8>(),
                );
                remaining = &remaining[6..];

                (
                    INesVersion::V2 {
                        console_type: console_type.ok_or(ParsingError::BadConsoleType)?,
                        submapper,
                        misc_rom_count,
                        default_expansion_device,
                    },
                    timing_mode,
                )
            }
            _ => return Err(ParsingError::BadVersion { version }),
        };

        assert!(remaining.len() % 8 == 0, "Parser misalignment");

        let mut roms = HashMap::default();

        if trainer {
            let mut trainer_rom = vec![0; 512];
            remaining
                .read_exact(&mut trainer_rom)
                .map_err(|_| ParsingError::EarlyEOF)?;

            roms.insert(RomType::Trainer, trainer_rom);
        }

        let mut prg_rom = vec![0; prg_bank_count as usize * PRG_BANK_SIZE];
        remaining
            .read_exact(&mut prg_rom)
            .map_err(|_| ParsingError::EarlyEOF)?;
        roms.insert(RomType::Prg, prg_rom);

        let mut chr_rom = vec![0; chr_bank_count as usize * CHR_BANK_SIZE];
        remaining
            .read_exact(&mut chr_rom)
            .map_err(|_| ParsingError::EarlyEOF)?;
        roms.insert(RomType::Chr, chr_rom);

        if !remaining.is_empty() {
            tracing::warn!("Extra data at end of ROM: {}", remaining.len());
        }

        Ok(Self {
            mapper,
            alternative_nametables,
            non_volatile_memory,
            mirroring,
            version,
            timing_mode,
            roms,
        })
    }

    pub fn prg_bank_count(&self) -> usize {
        self.roms
            .get(&RomType::Prg)
            .map(|rom| rom.len() / PRG_BANK_SIZE)
            .unwrap_or(1)
    }

    pub fn chr_bank_count(&self) -> usize {
        self.roms
            .get(&RomType::Chr)
            .map(|rom| rom.len() / CHR_BANK_SIZE)
            .unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {}
