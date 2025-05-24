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
