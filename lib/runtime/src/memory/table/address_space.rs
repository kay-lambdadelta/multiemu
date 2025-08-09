use crate::{component::ComponentId, memory::Address};
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use std::{
    hash::{Hash, Hasher},
    num::NonZero,
    ops::RangeInclusive,
    sync::RwLock,
    vec::Vec,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct AddressSpaceHandle(NonZero<u16>);

impl AddressSpaceHandle {
    pub fn new(id: u16) -> Option<Self> {
        NonZero::new(id).map(AddressSpaceHandle)
    }
}

impl Hash for AddressSpaceHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u16(self.0.get());
    }
}

impl IsEnabled for AddressSpaceHandle {}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub struct TableEntry {
    pub component_id: ComponentId,
    pub component_assigned_range: RangeInclusive<Address>,
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    read_members: RwLock<RangeInclusiveMap<Address, ComponentId>>,
    write_members: RwLock<RangeInclusiveMap<Address, ComponentId>>,
}

impl AddressSpace {
    pub fn new(width_mask: Address) -> Self {
        Self {
            width_mask,
            read_members: Default::default(),
            write_members: Default::default(),
        }
    }

    /// Removes all memory maps for a component_id and remaps it like so
    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommands>) {
        let mut read_members = self.read_members.write().unwrap();
        let mut write_members = self.write_members.write().unwrap();

        for command in commands {
            match command {
                MemoryRemappingCommands::Remove { range, types } => {
                    tracing::debug!("Removing memory range {:#04x?} from address space", range);

                    if types.contains(&MemoryType::Read) {
                        read_members.remove(range.clone());
                    }

                    if types.contains(&MemoryType::Write) {
                        write_members.remove(range.clone());
                    }
                }
                MemoryRemappingCommands::Add {
                    range,
                    component_id,
                    types,
                } => {
                    tracing::debug!(
                        "Mapping memory component_id {:?} to address range {:#04x?}",
                        component_id,
                        range
                    );

                    if types.contains(&MemoryType::Read) {
                        read_members.insert(range.clone(), component_id);
                    }

                    if types.contains(&MemoryType::Write) {
                        write_members.insert(range.clone(), component_id);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn visit_read_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        for (component_assigned_range, component_id) in self
            .read_members
            .read()
            .unwrap()
            .overlapping(accessing_range.clone())
        {
            callback(*component_id, component_assigned_range)?
        }

        Ok(())
    }

    #[inline]
    pub fn visit_write_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        for (component_assigned_range, component_id) in self
            .write_members
            .read()
            .unwrap()
            .overlapping(accessing_range.clone())
        {
            callback(*component_id, component_assigned_range)?
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum MemoryType {
    Read,
    Write,
}

#[derive(Debug)]
pub enum MemoryRemappingCommands {
    Remove {
        range: RangeInclusive<Address>,
        types: Vec<MemoryType>,
    },
    Add {
        range: RangeInclusive<Address>,
        component_id: ComponentId,
        types: Vec<MemoryType>,
    },
}
