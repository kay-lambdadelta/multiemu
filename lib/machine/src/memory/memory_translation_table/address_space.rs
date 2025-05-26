use super::MemoryHandle;
use crate::memory::Address;
use rangemap::RangeInclusiveMap;
use std::{num::NonZero, ops::RangeInclusive};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct AddressSpaceHandle(NonZero<u16>);

impl AddressSpaceHandle {
    pub(crate) const fn new(id: NonZero<u16>) -> Self {
        Self(id)
    }

    pub(crate) const fn get(&self) -> usize {
        (self.0.get() as usize) - 1
    }
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    pub read_members: RangeInclusiveMap<Address, MemoryHandle>,
    pub write_members: RangeInclusiveMap<Address, MemoryHandle>,
}

impl AddressSpace {
    /// Removes all memory maps for a handle and remaps it like so
    pub fn remap_memory(
        &mut self,
        handle: MemoryHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<usize>>,
    ) {
        // go through and remove entries with this handle

        let mut removals = Vec::new();
        for (addresses, memory_handle) in self.read_members.iter() {
            if memory_handle == &handle {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.read_members.remove(addresses.clone());
        }

        let mut removals = Vec::new();
        for (addresses, memory_handle) in self.write_members.iter() {
            if memory_handle == &handle {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.write_members.remove(addresses.clone());
        }

        for addresses in mapping {
            self.read_members.insert(addresses.clone(), handle);
            self.write_members.insert(addresses, handle);
        }
    }

    /// Removes all write memory maps for a handle and remaps it like so
    pub fn remap_write_memory(
        &mut self,
        handle: MemoryHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<usize>>,
    ) {
        // go through and remove entries with this handle

        let mut removals = Vec::new();
        for (addresses, memory_handle) in self.write_members.iter() {
            if memory_handle == &handle {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.write_members.remove(addresses.clone());
        }

        for addresses in mapping {
            self.write_members.insert(addresses.clone(), handle);
        }
    }

    /// Removes all read memory maps for a handle and remaps it like so
    pub fn remap_read_memory(
        &mut self,
        handle: MemoryHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<usize>>,
    ) {
        // go through and remove entries with this handle

        let mut removals = Vec::new();
        for (addresses, memory_handle) in self.read_members.iter() {
            if memory_handle == &handle {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.read_members.remove(addresses.clone());
        }

        for addresses in mapping {
            self.read_members.insert(addresses.clone(), handle);
        }
    }
}
