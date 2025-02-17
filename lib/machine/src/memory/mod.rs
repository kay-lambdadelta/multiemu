use serde::{Deserialize, Serialize};
use std::ops::Range;

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
pub const MEMORY_ACCESS_SIZE: Range<usize> = const {
    let mut min = VALID_MEMORY_ACCESS_SIZES[0];
    let mut index = 0;
    while index < VALID_MEMORY_ACCESS_SIZES.len() {
        if VALID_MEMORY_ACCESS_SIZES[index] < min {
            min = VALID_MEMORY_ACCESS_SIZES[index];
        }
        index += 1;
    }

    let mut max = VALID_MEMORY_ACCESS_SIZES[0];
    let mut index = 0;
    while index < VALID_MEMORY_ACCESS_SIZES.len() {
        if VALID_MEMORY_ACCESS_SIZES[index] > max {
            max = VALID_MEMORY_ACCESS_SIZES[index];
        }
        index += 1;
    }

    min..max
};
