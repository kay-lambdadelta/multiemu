use crate::{component::ComponentId, memory::Address};
use rangemap::RangeInclusiveMap;
use std::{num::NonZero, ops::RangeInclusive, vec::Vec};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct AddressSpaceHandle(NonZero<u16>);

impl AddressSpaceHandle {
    pub fn new(id: u16) -> Option<Self> {
        NonZero::new(id).map(AddressSpaceHandle)
    }

    pub fn get(self) -> u16 {
        self.0.get() - 1
    }
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    pub read_members: RangeInclusiveMap<Address, ComponentId>,
    pub write_members: RangeInclusiveMap<Address, ComponentId>,
}

impl AddressSpace {
    /// Removes all memory maps for a component_id and remaps it like so
    pub fn remap_memory(
        &mut self,
        component_id: ComponentId,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        // go through and remove entries with this component_id

        let mut removals = Vec::new();
        for (addresses, stored_component_id) in self.read_members.iter() {
            if stored_component_id == &component_id {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.read_members.remove(addresses.clone());
        }

        let mut removals = Vec::new();
        for (addresses, stored_component_id) in self.write_members.iter() {
            if stored_component_id == &component_id {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.write_members.remove(addresses.clone());
        }

        for addresses in mapping {
            tracing::debug!(
                "Mapping memory component_id {:?} to address range {:#04x?}",
                component_id,
                addresses
            );

            self.read_members.insert(addresses.clone(), component_id);
            self.write_members.insert(addresses, component_id);
        }
    }

    /// Removes all write memory maps for a component_id and remaps it like so
    pub fn remap_write_memory(
        &mut self,
        component_id: ComponentId,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        // go through and remove entries with this component_id

        let mut removals = Vec::new();
        for (addresses, stored_component_id) in self.write_members.iter() {
            if stored_component_id == &component_id {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.write_members.remove(addresses.clone());
        }

        for addresses in mapping {
            tracing::debug!(
                "Mapping write memory component_id {:?} to address range {:#04x?}",
                component_id,
                addresses
            );

            self.write_members.insert(addresses.clone(), component_id);
        }
    }

    /// Removes all read memory maps for a component_id and remaps it like so
    pub fn remap_read_memory(
        &mut self,
        component_id: ComponentId,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        // go through and remove entries with this component_id

        let mut removals = Vec::new();
        for (addresses, stored_component_id) in self.read_members.iter() {
            if stored_component_id == &component_id {
                removals.push(addresses.clone());
            }
        }
        for addresses in removals {
            self.read_members.remove(addresses.clone());
        }

        for addresses in mapping {
            tracing::debug!(
                "Mapping read memory component_id {:?} to address range {:#04x?}",
                component_id,
                addresses
            );

            self.read_members.insert(addresses.clone(), component_id);
        }
    }
}
