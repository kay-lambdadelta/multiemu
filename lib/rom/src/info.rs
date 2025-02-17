use super::system::GameSystem;
use isolang::Language;
use redb::{TypeName, Value};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashSet;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RomInfo {
    pub name: String,
    pub system: GameSystem,
    pub languages: HashSet<Language>,
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
