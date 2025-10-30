use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, fmt::Display, iter::once, path::Path, str::FromStr, sync::LazyLock,
};
use strum::{EnumIter, IntoEnumIterator};

mod extension;
mod guess;

#[derive(
    Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
/// Game systems organized by vendor
pub enum MachineId {
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

impl MachineId {
    /// Iterate over all possible game systems
    pub fn iter() -> impl Iterator<Item = Self> {
        NintendoSystem::iter()
            .map(MachineId::Nintendo)
            .chain(SegaSystem::iter().map(MachineId::Sega))
            .chain(SonySystem::iter().map(MachineId::Sony))
            .chain(AtariSystem::iter().map(MachineId::Atari))
            .chain(OtherSystem::iter().map(MachineId::Other))
            .chain(once(MachineId::Unknown))
    }

    /// Get a well known file extension for the files this system supports
    pub fn extension(self) -> Option<&'static str> {
        extension::get_extension(self)
    }

    /// Attempt to guess the game system from several heuristics including file extension and file contents
    pub fn guess(rom_path: impl AsRef<Path>) -> Option<Self> {
        guess::guess_system(rom_path)
    }

    /// Converts the name to a "Nointro" convention string
    pub fn to_nointro_string(&self) -> &'static str {
        match self {
            MachineId::Nintendo(NintendoSystem::GameBoy) => "Nintendo - Game Boy",
            MachineId::Nintendo(NintendoSystem::GameBoyColor) => "Nintendo - Game Boy Color",
            MachineId::Nintendo(NintendoSystem::GameBoyAdvance) => "Nintendo - Game Boy Advance",
            MachineId::Nintendo(NintendoSystem::GameCube) => "Nintendo - Nintendo GameCube",
            MachineId::Nintendo(NintendoSystem::Wii) => "Nintendo - Wii",
            MachineId::Nintendo(NintendoSystem::WiiU) => "Nintendo - Wii U",
            MachineId::Nintendo(NintendoSystem::SuperNintendoEntertainmentSystem) => {
                "Nintendo - Super Nintendo Entertainment System"
            }
            MachineId::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => {
                "Nintendo - Nintendo Entertainment System"
            }
            MachineId::Nintendo(NintendoSystem::Nintendo64) => "Nintendo - Nintendo 64",
            MachineId::Nintendo(NintendoSystem::NintendoDS) => "Nintendo - Nintendo DS",
            MachineId::Nintendo(NintendoSystem::NintendoDSi) => "Nintendo - Nintendo DSi",
            MachineId::Nintendo(NintendoSystem::Nintendo3DS) => "Nintendo - Nintendo 3DS",
            MachineId::Nintendo(NintendoSystem::PokemonMini) => "Nintendo - Pokemon Mini",
            MachineId::Nintendo(NintendoSystem::VirtualBoy) => "Nintendo - Virtual Boy",
            MachineId::Sony(SonySystem::Playstation) => "Sony - PlayStation",
            MachineId::Sony(SonySystem::Playstation2) => "Sony - PlayStation 2",
            MachineId::Sony(SonySystem::Playstation3) => "Sony - PlayStation 3",
            MachineId::Sony(SonySystem::PlaystationPortable) => "Sony - PlayStation Portable",
            MachineId::Sony(SonySystem::PlaystationVita) => "Sony - PlayStation Vita",
            MachineId::Sega(SegaSystem::MasterSystem) => "Sega - Master System",
            MachineId::Sega(SegaSystem::GameGear) => "Sega - Game Gear",
            MachineId::Sega(SegaSystem::Genesis) => "Sega - Mega Drive - Genesis",
            MachineId::Sega(SegaSystem::SegaCD) => "Sega - Sega CD",
            MachineId::Sega(SegaSystem::Sega32X) => "Sega - 32X",
            MachineId::Other(OtherSystem::Chip8) => "Other - Chip8",
            MachineId::Atari(AtariSystem::Atari2600) => "Atari - 2600",
            MachineId::Atari(AtariSystem::Atari5200) => "Atari - 5200",
            MachineId::Atari(AtariSystem::Atari7800) => "Atari - 7800",
            MachineId::Atari(AtariSystem::Lynx) => "Atari - Atari Lynx",
            MachineId::Atari(AtariSystem::Jaguar) => "Atari - Jaguar",
            MachineId::Unknown => "Unknown",
        }
    }

    /// FIXME: This is written as stupidly as it could be
    pub fn from_nointro_str(s: &str) -> Result<Self, String> {
        let original_s = s;

        let s = strip_brackets_and_parens(
            &s.replace("Non-Redump -", "")
                .replace("Unofficial -", "")
                .replace("- BIOS Images", ""),
        )
        .trim()
        .to_lowercase()
        .replace(' ', "");

        static SYSTEMS_AS_NOINTRO_STRING: LazyLock<HashMap<String, MachineId>> =
            LazyLock::new(|| {
                MachineId::iter()
                    .map(|system| {
                        (
                            system.to_nointro_string().to_lowercase().replace(' ', ""),
                            system,
                        )
                    })
                    .collect()
            });

        if let Some(system) = SYSTEMS_AS_NOINTRO_STRING.get(&s) {
            return Ok(*system);
        }

        let company_names = [
            NintendoSystem::NOINTRO_NAME,
            SegaSystem::NOINTRO_NAME,
            SonySystem::NOINTRO_NAME,
            AtariSystem::NOINTRO_NAME,
            OtherSystem::NOINTRO_NAME,
        ];

        for company_name in company_names.map(str::to_lowercase) {
            if let Some(index) = s.rfind(&company_name) {
                let mut s_without_company = s.clone();
                s_without_company.replace_range(index..index + company_name.len(), "");

                if let Some(system) = SYSTEMS_AS_NOINTRO_STRING.get(&s_without_company) {
                    return Ok(*system);
                }
            }

            for (system_string, system) in SYSTEMS_AS_NOINTRO_STRING.iter() {
                if let Some(index) = system_string.rfind(&company_name) {
                    let mut system_string_without_company = system_string.clone();
                    system_string_without_company
                        .replace_range(index..index + company_name.len(), "");

                    if s == system_string_without_company {
                        return Ok(*system);
                    }
                }
            }
        }

        Err(format!("Unknown system: {original_s}"))
    }
}

impl AsRef<str> for MachineId {
    fn as_ref(&self) -> &str {
        match self {
            // Nintendo
            MachineId::Nintendo(NintendoSystem::GameBoy) => "nintendo~game-boy",
            MachineId::Nintendo(NintendoSystem::GameBoyColor) => "nintendo~game-boy-color",
            MachineId::Nintendo(NintendoSystem::GameBoyAdvance) => "nintendo~game-boy-advance",
            MachineId::Nintendo(NintendoSystem::GameCube) => "nintendo~nintendo-gamecube",
            MachineId::Nintendo(NintendoSystem::Wii) => "nintendo~wii",
            MachineId::Nintendo(NintendoSystem::WiiU) => "nintendo~wii-u",
            MachineId::Nintendo(NintendoSystem::SuperNintendoEntertainmentSystem) => {
                "nintendo~super-nintendo-entertainment-system"
            }
            MachineId::Nintendo(NintendoSystem::NintendoEntertainmentSystem) => {
                "nintendo~nintendo-entertainment-system"
            }
            MachineId::Nintendo(NintendoSystem::Nintendo64) => "nintendo~nintendo-64",
            MachineId::Nintendo(NintendoSystem::NintendoDS) => "nintendo~nintendo-ds",
            MachineId::Nintendo(NintendoSystem::NintendoDSi) => "nintendo~nintendo-dsi",
            MachineId::Nintendo(NintendoSystem::Nintendo3DS) => "nintendo~nintendo-3ds",
            MachineId::Nintendo(NintendoSystem::PokemonMini) => "nintendo~pokemon-mini",
            MachineId::Nintendo(NintendoSystem::VirtualBoy) => "nintendo~virtual-boy",

            MachineId::Sony(SonySystem::Playstation) => "sony~playstation",
            MachineId::Sony(SonySystem::Playstation2) => "sony~playstation-2",
            MachineId::Sony(SonySystem::Playstation3) => "sony~playstation-3",
            MachineId::Sony(SonySystem::PlaystationPortable) => "sony~playstation-portable",
            MachineId::Sony(SonySystem::PlaystationVita) => "sony~playstation-vita",

            MachineId::Sega(SegaSystem::MasterSystem) => "sega~master-system",
            MachineId::Sega(SegaSystem::GameGear) => "sega~game-gear",
            MachineId::Sega(SegaSystem::Genesis) => "sega~mega-drive-genesis",
            MachineId::Sega(SegaSystem::SegaCD) => "sega~sega-cd",
            MachineId::Sega(SegaSystem::Sega32X) => "sega~32x",

            MachineId::Atari(AtariSystem::Atari2600) => "atari~2600",
            MachineId::Atari(AtariSystem::Atari5200) => "atari~5200",
            MachineId::Atari(AtariSystem::Atari7800) => "atari~7800",
            MachineId::Atari(AtariSystem::Lynx) => "atari~lynx",
            MachineId::Atari(AtariSystem::Jaguar) => "atari~jaguar",

            MachineId::Other(OtherSystem::Chip8) => "other~chip8",
            MachineId::Unknown => "unknown",
        }
    }
}

impl Display for MachineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl FromStr for MachineId {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::iter()
            .find(|system| system.as_ref() == s)
            .ok_or("Could not parse")
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
    pub const NOINTRO_NAME: &str = "Nintendo";
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
    pub const NOINTRO_NAME: &str = "Sega";
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
    pub const NOINTRO_NAME: &str = "Sony";
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
    pub const NOINTRO_NAME: &str = "Other";
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
    pub const NOINTRO_NAME: &str = "Atari";
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
