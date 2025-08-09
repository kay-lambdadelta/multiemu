use crate::memory::{Address, AddressSpaceHandle};
use rangemap::RangeInclusiveSet;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct MemoryMetadata {
    pub read: HashMap<AddressSpaceHandle, RangeInclusiveSet<Address>>,
    pub write: HashMap<AddressSpaceHandle, RangeInclusiveSet<Address>>,
}
