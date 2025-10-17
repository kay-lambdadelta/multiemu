use crate::program::RomId;
use multiemu_locale::Iso639Alpha3;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::{BTreeMap, BTreeSet};
use versions::Versioning;

/// Paths are unixlike

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Filesystem {
    Single { rom_id: RomId, file_name: String },
    Complex(BTreeMap<RomId, BTreeSet<String>>),
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Information about a program, for the database
pub enum ProgramInfo {
    /// Version 0
    #[serde(rename = "0")]
    V0 {
        /// Identifiable names of the program
        ///
        /// Preferably these will be the names associated with the below languages, in their original script
        names: BTreeSet<String>,
        /// Filesystem of the program
        filesystem: Filesystem,
        /// The language this program is associated with
        ///
        /// Note that this is the languages a coherent title supports
        ///
        /// If alternate files are required a different database entry is required
        languages: BTreeSet<Iso639Alpha3>,
        #[serde_as(as = "Option<DisplayFromStr>")]
        version: Option<Versioning>,
    },
}

impl ProgramInfo {
    /// Returns the name of the program
    pub fn names(&self) -> &BTreeSet<String> {
        match self {
            ProgramInfo::V0 { names, .. } => names,
        }
    }

    /// Returns the path of the program
    pub fn filesystem(&self) -> &Filesystem {
        match self {
            ProgramInfo::V0 { filesystem, .. } => filesystem,
        }
    }

    /// Converts this to the latest version
    pub fn mitigate(self) -> Self {
        match self {
            ProgramInfo::V0 { .. } => self,
        }
    }
}

impl Value for ProgramInfo {
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
        TypeName::new("program_info")
    }
}

impl Key for ProgramInfo {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        let data1 = Self::from_bytes(data1);
        let data2 = Self::from_bytes(data2);

        data1.cmp(&data2)
    }
}
