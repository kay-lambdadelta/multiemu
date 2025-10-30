use data_encoding::HEXLOWER_PERMISSIVE;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{cmp::Ordering, fmt::Display, io::Read, str::FromStr};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Hash of a ROM, serves as its identification within emulator (this is a sha1 hash)
pub struct RomId(pub [u8; 20]);

impl RomId {
    // I would greatly prefer if sha256 was the one used for identity but some rom datsheets only have sha1 or worse

    /// Calculate the rom id (SHA-1) of a rom
    pub fn calculate_id(data: &mut impl Read) -> Result<Self, std::io::Error> {
        let mut hasher = Sha1::new();
        std::io::copy(data, &mut hasher)?;
        Ok(Self(hasher.finalize().into()))
    }
}

impl Display for RomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", HEXLOWER_PERMISSIVE.encode(&self.0))
    }
}

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum RomIdParsingError {
    #[error("{0}")]
    DataEncoding(#[from] data_encoding::DecodeError),
    #[error("Hash did not meet the requirements for this function")]
    InvalidHash,
}

impl FromStr for RomId {
    type Err = RomIdParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = HEXLOWER_PERMISSIVE.decode(s.as_bytes())?;

        Ok(Self(
            bytes
                .try_into()
                .map_err(|_| RomIdParsingError::InvalidHash)?,
        ))
    }
}

impl Value for RomId {
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
        TypeName::new("rom_id")
    }
}

impl Key for RomId {
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        data1.cmp(data2)
    }
}
