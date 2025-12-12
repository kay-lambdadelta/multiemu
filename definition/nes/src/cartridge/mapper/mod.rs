use serde::{Deserialize, Serialize};

pub mod mmc1;
pub mod nrom;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum Mapper {
    NRom,
    Mmc1,
}
