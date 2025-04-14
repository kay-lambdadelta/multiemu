use serde::{Deserialize, Serialize};
use std::{
    cell::LazyCell, collections::HashMap, fmt::Display, iter::once, path::Path, str::FromStr,
};
use strum::{EnumIter, IntoEnumIterator};

mod extension;
mod guess;

#[derive(
    Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
/// All possible game systems and more
pub enum GameSystem {
    Nintendo(NintendoSystem),
    Sega(SegaSystem),
    Sony(SonySystem),
    Atari(AtariSystem),
    Other(OtherSystem),
    #[default]
    Unknown,
}

impl GameSystem {
    /// Iterate over all possible game systems
    pub fn iter() -> impl Iterator<Item = Self> {
        NintendoSystem::iter()
            .map(GameSystem::Nintendo)
            .chain(SegaSystem::iter().map(GameSystem::Sega))
            .chain(SonySystem::iter().map(GameSystem::Sony))
            .chain(AtariSystem::iter().map(GameSystem::Atari))
            .chain(OtherSystem::iter().map(GameSystem::Other))
            .chain(once(GameSystem::Unknown))
    }

    /// Get a well known file extension for the files this system supports
    pub fn extension(self) -> Option<&'static str> {
        extension::get_extension(self)
    }

    pub fn guess(rom_path: impl AsRef<Path>) -> Option<Self> {
        guess::guess_system(rom_path)
    }
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter,
)]
/// All Nintendo systems
pub enum NintendoSystem {
    GameBoy,
    GameBoyColor,
    GameBoyAdvance,
    GameCube,
    Wii,
    WiiU,
    NintendoEntertainmentSystem,
    SuperNintendoEntertainmentSystem,
    Nintendo64,
    NintendoDS,
    NintendoDSi,
    Nintendo3DS,
    PokemonMini,
    VirtualBoy,
}

impl NintendoSystem {
    pub const NAME: &str = "Nintendo";
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter,
)]
/// All Sega systems
pub enum SegaSystem {
    MasterSystem,
    GameGear,
    Genesis,
    Sega32X,
    SegaCD,
}

impl SegaSystem {
    pub const NAME: &str = "Sega";
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter,
)]
/// All Sony systems
pub enum SonySystem {
    Playstation,
    Playstation2,
    Playstation3,
    PlaystationPortable,
    PlaystationVita,
}

impl SonySystem {
    pub const NAME: &str = "Sony";
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter,
)]
/// Some random assorted other systems
pub enum OtherSystem {
    Chip8,
}

impl OtherSystem {
    pub const NAME: &str = "Other";
}

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter,
)]
/// All Atari systems
pub enum AtariSystem {
    Atari2600,
    Atari5200,
    Atari7800,
    Lynx,
    Jaguar,
}

impl AtariSystem {
    pub const NAME: &str = "Atari";
}

impl FromStr for GameSystem {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let original_s = s;

        let s = strip_brackets_and_parens(
            &s.replace("Non-Redump -", "")
                .replace("Unofficial -", "")
                .replace("- BIOS Images", ""),
        )
        .trim()
        .to_lowercase()
        .replace(' ', "");

        thread_local! {
            static SYSTEMS_AS_STRINGS: LazyCell<HashMap<String, GameSystem>> = LazyCell::new(
                || {
                    GameSystem::iter()
                        .map(|system| (system.to_string().to_lowercase().replace(' ', ""), system))
                    .collect()
                }
            )
        }

        if let Some(system) = SYSTEMS_AS_STRINGS.with(|systems| {
            if let Some(system) = systems.get(&s) {
                return Some(*system);
            }

            let company_names = [
                NintendoSystem::NAME,
                SegaSystem::NAME,
                SonySystem::NAME,
                AtariSystem::NAME,
                OtherSystem::NAME,
            ];

            for company_name in company_names.map(|company_name| company_name.to_lowercase()) {
                if let Some(index) = s.rfind(&company_name) {
                    let mut s_without_company = s.clone();
                    s_without_company.replace_range(index..index + company_name.len(), "");

                    if let Some(system) = systems.get(&s_without_company) {
                        return Some(*system);
                    }
                }

                for (system_string, system) in systems.iter() {
                    if let Some(index) = system_string.rfind(&company_name) {
                        let mut system_string_without_company = system_string.clone();
                        system_string_without_company
                            .replace_range(index..index + company_name.len(), "");

                        if s == system_string_without_company {
                            return Some(*system);
                        }
                    }
                }
            }

            None
        }) {
            return Ok(system);
        }

        Err(format!("Unknown system: {}", original_s))
    }
}

/// Exports a well formed No-Intro style system name
impl Display for GameSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameSystem::Nintendo(NintendoSystem::GameBoy) => write!(f, "Nintendo - Game Boy"),
            GameSystem::Nintendo(NintendoSystem::GameBoyColor) => {
                write!(f, "Nintendo - Game Boy Color")
            }
            GameSystem::Nintendo(NintendoSystem::GameBoyAdvance) => {
                write!(f, "Nintendo - Game Boy Advance")
            }
            GameSystem::Nintendo(NintendoSystem::GameCube) => {
                write!(f, "Nintendo - Nintendo GameCube")
            }
            GameSystem::Nintendo(NintendoSystem::Wii) => write!(f, "Nintendo - Wii"),
            GameSystem::Nintendo(NintendoSystem::WiiU) => write!(f, "Nintendo - Wii U"),
            GameSystem::Nintendo(NintendoSystem::SuperNintendoEntertainmentSystem) => {
                write!(f, "Nintendo - Super Nintendo Entertainment System")
            }
            GameSystem::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => {
                write!(f, "Nintendo - Nintendo Entertainment System")
            }
            GameSystem::Nintendo(NintendoSystem::Nintendo64) => write!(f, "Nintendo - Nintendo 64"),
            GameSystem::Nintendo(NintendoSystem::NintendoDS) => write!(f, "Nintendo - Nintendo DS"),
            GameSystem::Nintendo(NintendoSystem::NintendoDSi) => {
                write!(f, "Nintendo - Nintendo DSi")
            }
            GameSystem::Nintendo(NintendoSystem::Nintendo3DS) => {
                write!(f, "Nintendo - Nintendo 3DS")
            }
            GameSystem::Nintendo(NintendoSystem::PokemonMini) => {
                write!(f, "Nintendo - Pokemon Mini")
            }
            GameSystem::Nintendo(NintendoSystem::VirtualBoy) => {
                write!(f, "Nintendo - Virtual Boy")
            }
            GameSystem::Sony(SonySystem::Playstation) => write!(f, "Sony - PlayStation"),
            GameSystem::Sony(SonySystem::Playstation2) => write!(f, "Sony - PlayStation 2"),
            GameSystem::Sony(SonySystem::Playstation3) => write!(f, "Sony - PlayStation 3"),
            GameSystem::Sony(SonySystem::PlaystationPortable) => {
                write!(f, "Sony - PlayStation Portable")
            }
            GameSystem::Sony(SonySystem::PlaystationVita) => write!(f, "Sony - PlayStation Vita"),
            GameSystem::Sega(SegaSystem::MasterSystem) => write!(f, "Sega - Master System"),
            GameSystem::Sega(SegaSystem::GameGear) => write!(f, "Sega - Game Gear"),
            GameSystem::Sega(SegaSystem::Genesis) => write!(f, "Sega - Mega Drive - Genesis"),
            GameSystem::Sega(SegaSystem::SegaCD) => write!(f, "Sega - Sega CD"),
            GameSystem::Sega(SegaSystem::Sega32X) => write!(f, "Sega - 32X"),
            GameSystem::Other(OtherSystem::Chip8) => write!(f, "Other - Chip8"),
            GameSystem::Atari(AtariSystem::Atari2600) => write!(f, "Atari - 2600"),
            GameSystem::Atari(AtariSystem::Atari5200) => write!(f, "Atari - 5200"),
            GameSystem::Atari(AtariSystem::Atari7800) => write!(f, "Atari - 7800"),
            GameSystem::Atari(AtariSystem::Lynx) => write!(f, "Atari - Atari Lynx"),
            GameSystem::Atari(AtariSystem::Jaguar) => write!(f, "Atari - Jaguar"),
            GameSystem::Unknown => write!(f, "Unknown"),
        }
    }
}

fn strip_brackets_and_parens(input: &str) -> String {
    let mut result = String::new();
    let mut skip_level = 0;

    for c in input.chars() {
        match c {
            '(' | '[' => skip_level += 1,
            ')' | ']' => {
                if skip_level > 0 {
                    skip_level -= 1;
                }
            }
            _ => {
                if skip_level == 0 {
                    result.push(c);
                }
            }
        }
    }

    result
}
