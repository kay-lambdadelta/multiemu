use serde::{Deserialize, Serialize};

pub mod callbacks;
pub mod memory_translation_table;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct AddressSpaceId(u8);

impl AddressSpaceId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }
}

pub const PAGE_SIZE: usize = 4096;
pub const VALID_MEMORY_ACCESS_SIZES: &[usize] = &[1, 2, 4, 8];
pub const MAX_MEMORY_ACCESS_SIZE: usize = 8;
