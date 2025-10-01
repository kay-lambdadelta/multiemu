use super::system::System;
use codes_iso_639::part_3::LanguageCode;
use codes_iso_3166::part_1::CountryCode;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::HashSet;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Information about a ROM, for the database
pub enum RomInfo {
    /// Version 0
    #[serde(rename = "0")]
    V0 {
        /// Identifiable name of the game
        name: String,
        /// The path of the ROM file
        path: Vec<String>,
        /// The system this ROM is associated with
        system: System,
        #[serde(default)]
        #[serde_as(as = "HashSet<DisplayFromStr>")]
        /// The languages this ROM is available in
        languages: HashSet<LanguageCode>,
        #[serde(default)]
        #[serde_as(as = "HashSet<DisplayFromStr>")]
        /// The regions this ROM is available in
        regions: HashSet<CountryCode>,
    },
}

impl RomInfo {
    /// Returns the name of the ROM
    pub fn name(&self) -> &str {
        match self {
            RomInfo::V0 { name, .. } => name,
        }
    }

    /// Returns the path of the ROM
    pub fn path(&self) -> &[String] {
        match self {
            RomInfo::V0 { path, .. } => path,
        }
    }

    /// Returns the system of the ROM
    pub fn system(&self) -> System {
        match self {
            RomInfo::V0 { system, .. } => *system,
        }
    }

    /// Returns the languages that the ROM supports
    pub fn languages(&self) -> &HashSet<LanguageCode> {
        match self {
            RomInfo::V0 { languages, .. } => languages,
        }
    }

    /// Returns the regions that the ROM supports
    pub fn regions(&self) -> &HashSet<CountryCode> {
        match self {
            RomInfo::V0 { regions, .. } => regions,
        }
    }

    /// Converts this to the latest version
    pub fn mitigate(self) -> Self {
        match self {
            RomInfo::V0 { .. } => self,
        }
    }
}

impl Value for RomInfo {
    type SelfType<'a> = Self;

    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::serde::decode_from_slice(data, bincode::config::standard())
            .unwrap()
            .0
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        bincode::serde::encode_to_vec(value, bincode::config::standard()).unwrap()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("rom_info")
    }
}

impl Key for RomInfo {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        let value1 = Self::from_bytes(data1);
        let value2 = Self::from_bytes(data2);

        // Only the path and name are unique
        value1.path().cmp(value2.path()).then_with(|| {
            value1
                .name()
                .to_lowercase()
                .cmp(&value2.name().to_lowercase())
        })
    }
}
