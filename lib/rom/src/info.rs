use super::system::GameSystem;
use crate::id::RomId;
use camino::Utf8PathBuf;
use isolang::Language;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeSet;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Information about a ROM, for the database
pub struct RomInfoV0 {
    pub name: String,
    pub file_name: Utf8PathBuf,
    pub system: GameSystem,
    #[serde(default)]
    pub languages: BTreeSet<Language>,
    #[serde(default)]
    pub dependencies: BTreeSet<RomId>,
}

impl Value for RomInfoV0 {
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

impl Key for RomInfoV0 {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        let value1 = Self::from_bytes(data1);
        let value2 = Self::from_bytes(data2);

        value1
            .file_name
            .cmp(&value2.file_name)
            .then_with(|| value1.name.to_lowercase().cmp(&value2.name.to_lowercase()))
    }
}
