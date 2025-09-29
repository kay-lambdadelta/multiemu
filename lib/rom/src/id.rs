use data_encoding::HEXLOWER_PERMISSIVE;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{cmp::Ordering, fmt::Display, io::Read, str::FromStr};

#[derive(
    Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
/// SHA-1 of a ROM, serves as its identification within emulator
pub struct RomId([u8; 20]);

impl RomId {
    /// Create from a given sha1 hash
    pub const fn new(hash: [u8; 20]) -> Self {
        Self(hash)
    }

    /// Calculate the ID manually
    pub fn calculate_id(mut data: impl Read) -> Result<Self, std::io::Error> {
        let mut hasher = Sha1::new();
        std::io::copy(&mut data, &mut hasher)?;
        Ok(Self(hasher.finalize().into()))
    }
}

impl AsRef<[u8]> for RomId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 20]> for RomId {
    fn from(value: [u8; 20]) -> Self {
        Self(value)
    }
}

impl Display for RomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", HEXLOWER_PERMISSIVE.encode(&self.0))
    }
}

impl FromStr for RomId {
    type Err = data_encoding::DecodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = HEXLOWER_PERMISSIVE.decode(s.as_bytes())?;
        Ok(Self(bytes.try_into().unwrap()))
    }
}

impl Value for RomId {
    type SelfType<'a> = Self;

    type AsBytes<'a> = &'a [u8];

    fn fixed_width() -> Option<usize> {
        Some(20)
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Self(data.try_into().unwrap())
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        &value.0
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("rom_id")
    }
}

impl Key for RomId {
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        data1.cmp(data2)
    }
}
