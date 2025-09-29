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
/// Game systems organized by vendor
pub enum System {
    /// Nintendo systems
    Nintendo(NintendoSystem),
    /// Sega systems
    Sega(SegaSystem),
    /// Sony systems
    Sony(SonySystem),
    /// Atari systems
    Atari(AtariSystem),
    /// Systems that do not fit in the above vendors
    Other(OtherSystem),
    #[default]
    /// Unspecified system
    Unknown,
}

impl System {
    /// Iterate over all possible game systems
    pub fn iter() -> impl Iterator<Item = Self> {
        NintendoSystem::iter()
            .map(System::Nintendo)
            .chain(SegaSystem::iter().map(System::Sega))
            .chain(SonySystem::iter().map(System::Sony))
            .chain(AtariSystem::iter().map(System::Atari))
            .chain(OtherSystem::iter().map(System::Other))
            .chain(once(System::Unknown))
    }

    /// Get a well known file extension for the files this system supports
    pub fn extension(self) -> Option<&'static str> {
        extension::get_extension(self)
    }

    /// Attempt to guess the game system from several heuristics including file extension and file contents
    pub fn guess(rom_path: impl AsRef<Path>) -> Option<Self> {
        guess::guess_system(rom_path)
    }
}

#[allow(missing_docs)]
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

#[allow(missing_docs)]
impl NintendoSystem {
    pub const NAME: &str = "Nintendo";
}

#[allow(missing_docs)]
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

#[allow(missing_docs)]
impl SegaSystem {
    pub const NAME: &str = "Sega";
}

#[allow(missing_docs)]
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

#[allow(missing_docs)]
impl SonySystem {
    pub const NAME: &str = "Sony";
}

#[allow(missing_docs)]
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter,
)]
/// Some random assorted other systems
pub enum OtherSystem {
    Chip8,
}

#[allow(missing_docs)]
impl OtherSystem {
    pub const NAME: &str = "Other";
}

#[allow(missing_docs)]
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

#[allow(missing_docs)]
impl AtariSystem {
    pub const NAME: &str = "Atari";
}

impl FromStr for System {
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
            static SYSTEMS_AS_STRINGS: LazyCell<HashMap<String, System>> = LazyCell::new(
                || {
                    System::iter()
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
impl Display for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            System::Nintendo(NintendoSystem::GameBoy) => write!(f, "Nintendo - Game Boy"),
            System::Nintendo(NintendoSystem::GameBoyColor) => {
                write!(f, "Nintendo - Game Boy Color")
            }
            System::Nintendo(NintendoSystem::GameBoyAdvance) => {
                write!(f, "Nintendo - Game Boy Advance")
            }
            System::Nintendo(NintendoSystem::GameCube) => {
                write!(f, "Nintendo - Nintendo GameCube")
            }
            System::Nintendo(NintendoSystem::Wii) => write!(f, "Nintendo - Wii"),
            System::Nintendo(NintendoSystem::WiiU) => write!(f, "Nintendo - Wii U"),
            System::Nintendo(NintendoSystem::SuperNintendoEntertainmentSystem) => {
                write!(f, "Nintendo - Super Nintendo Entertainment System")
            }
            System::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => {
                write!(f, "Nintendo - Nintendo Entertainment System")
            }
            System::Nintendo(NintendoSystem::Nintendo64) => write!(f, "Nintendo - Nintendo 64"),
            System::Nintendo(NintendoSystem::NintendoDS) => write!(f, "Nintendo - Nintendo DS"),
            System::Nintendo(NintendoSystem::NintendoDSi) => {
                write!(f, "Nintendo - Nintendo DSi")
            }
            System::Nintendo(NintendoSystem::Nintendo3DS) => {
                write!(f, "Nintendo - Nintendo 3DS")
            }
            System::Nintendo(NintendoSystem::PokemonMini) => {
                write!(f, "Nintendo - Pokemon Mini")
            }
            System::Nintendo(NintendoSystem::VirtualBoy) => {
                write!(f, "Nintendo - Virtual Boy")
            }
            System::Sony(SonySystem::Playstation) => write!(f, "Sony - PlayStation"),
            System::Sony(SonySystem::Playstation2) => write!(f, "Sony - PlayStation 2"),
            System::Sony(SonySystem::Playstation3) => write!(f, "Sony - PlayStation 3"),
            System::Sony(SonySystem::PlaystationPortable) => {
                write!(f, "Sony - PlayStation Portable")
            }
            System::Sony(SonySystem::PlaystationVita) => write!(f, "Sony - PlayStation Vita"),
            System::Sega(SegaSystem::MasterSystem) => write!(f, "Sega - Master System"),
            System::Sega(SegaSystem::GameGear) => write!(f, "Sega - Game Gear"),
            System::Sega(SegaSystem::Genesis) => write!(f, "Sega - Mega Drive - Genesis"),
            System::Sega(SegaSystem::SegaCD) => write!(f, "Sega - Sega CD"),
            System::Sega(SegaSystem::Sega32X) => write!(f, "Sega - 32X"),
            System::Other(OtherSystem::Chip8) => write!(f, "Other - Chip8"),
            System::Atari(AtariSystem::Atari2600) => write!(f, "Atari - 2600"),
            System::Atari(AtariSystem::Atari5200) => write!(f, "Atari - 5200"),
            System::Atari(AtariSystem::Atari7800) => write!(f, "Atari - 7800"),
            System::Atari(AtariSystem::Lynx) => write!(f, "Atari - Atari Lynx"),
            System::Atari(AtariSystem::Jaguar) => write!(f, "Atari - Jaguar"),
            System::Unknown => write!(f, "Unknown"),
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
