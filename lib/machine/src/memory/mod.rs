use serde::{Deserialize, Serialize};

pub mod callbacks;
pub mod memory_translation_table;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct AddressSpaceHandle(u16);

impl AddressSpaceHandle {
    pub(crate) const fn new(id: u16) -> Self {
        Self(id)
    }
}

pub type Address = usize;

pub const PAGE_SIZE: usize = 4096;
pub const VALID_MEMORY_ACCESS_SIZES: &[usize] = &[1, 2, 4, 8];
pub const MAX_MEMORY_ACCESS_SIZE: usize = 8;

/// What layer this memory callback belongs to
pub type MemoryLayer = u8;
pub const DEFAULT_MEMORY_LAYER: u8 = 0;
