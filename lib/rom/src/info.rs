use super::system::GameSystem;
use crate::id::RomId;
use isolang::Language;
use redb::{TypeName, Value};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeSet;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Information about a ROM, for the database
pub struct RomInfo {
    pub name: String,
    pub system: GameSystem,
    #[serde(default)]
    pub languages: BTreeSet<Language>,
    #[serde(default)]
    pub dependencies: BTreeSet<RomId>,
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
        rmp_serde::from_slice(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        rmp_serde::to_vec_named(value).unwrap()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("rom_info")
    }
}
