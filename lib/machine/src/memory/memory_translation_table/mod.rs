use super::{
    Address, AddressSpaceHandle,
    callbacks::{ReadMemory, WriteMemory},
};
use bitvec::{field::BitField, order::Lsb0};
use rangemap::RangeInclusiveMap;
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

mod read;
mod write;

pub use read::*;
pub use write::*;

trait ReadWriteMemory: ReadMemory + WriteMemory {}
impl<M: ReadMemory + WriteMemory> ReadWriteMemory for M {}

#[derive(Debug)]
enum StoredCallback {
    Read(Box<dyn ReadMemory>),
    Write(Box<dyn WriteMemory>),
    ReadWrite(Box<dyn ReadWriteMemory>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct MemoryHandle(u16);

impl MemoryHandle {
    pub(crate) const fn new(id: u16) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
struct AddressSpaceInfo {
    width_mask: Address,
    #[allow(unused)]
    name: &'static str,
    read_members: RangeInclusiveMap<Address, MemoryHandle>,
    write_members: RangeInclusiveMap<Address, MemoryHandle>,
}

#[derive(Default)]
struct MemoryTranslationTableImpl {
    current_address_space_id: u16,
    address_spaces: HashMap<AddressSpaceHandle, AddressSpaceInfo, FxBuildHasher>,

    current_memory_id: u16,
    memories: HashMap<MemoryHandle, StoredCallback, FxBuildHasher>,
}

impl Debug for MemoryTranslationTableImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[allow(unused)]
        #[derive(Debug)]
        struct AddressMap<'a> {
            map: Vec<(RangeInclusive<usize>, &'a StoredCallback)>,
        }

        let helper: HashMap<_, _> = self
            .address_spaces
            .iter()
            .map(|(address_space_handle, info)| {
                (
                    address_space_handle,
                    AddressMap {
                        map: info
                            .write_members
                            .iter()
                            .map(|(range, memory_handle)| {
                                (range.clone(), self.memories.get(memory_handle).unwrap())
                            })
                            .collect(),
                    },
                )
            })
            .collect();

        f.debug_tuple("MemoryTranslationTable")
            .field(&helper)
            .finish()
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemoryTranslationTable(Arc<RwLock<MemoryTranslationTableImpl>>);

impl MemoryTranslationTable {
    pub fn insert_address_space(&self, name: &'static str, width: u8) -> AddressSpaceHandle {
        let mut impl_guard = self.0.write().unwrap();

        let id = AddressSpaceHandle::new(impl_guard.current_address_space_id);
        impl_guard.current_address_space_id = impl_guard
            .current_address_space_id
            .checked_add(1)
            .expect("Too many address spaces");

        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..width as usize].fill(true);
        let width_mask = mask.load();

        impl_guard.address_spaces.insert(
            id,
            AddressSpaceInfo {
                width_mask,
                name,
                read_members: RangeInclusiveMap::default(),
                write_members: RangeInclusiveMap::default(),
            },
        );

        id
    }

    pub fn insert_memory<M: ReadMemory + WriteMemory>(
        &self,
        memory: M,
        mappings: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> MemoryHandle {
        let mut impl_guard = self.0.write().unwrap();

        let id = MemoryHandle::new(impl_guard.current_memory_id);
        impl_guard.current_memory_id = impl_guard
            .current_memory_id
            .checked_add(1)
            .expect("Too many memories");

        impl_guard
            .memories
            .insert(id, StoredCallback::ReadWrite(Box::new(memory)));

        for (address_space, addresses) in mappings {
            let address_spaces = impl_guard
                .address_spaces
                .get_mut(&address_space)
                .expect("Non existant address space");
            address_spaces.read_members.insert(addresses.clone(), id);
            address_spaces.write_members.insert(addresses, id);
        }

        id
    }
}
