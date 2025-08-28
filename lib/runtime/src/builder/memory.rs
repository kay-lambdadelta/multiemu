use crate::memory::{Address, AddressSpaceId};
use rangemap::RangeInclusiveSet;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct MemoryMetadata {
    pub read: HashMap<AddressSpaceId, RangeInclusiveSet<Address>>,
    pub write: HashMap<AddressSpaceId, RangeInclusiveSet<Address>>,
}
