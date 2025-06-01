use super::{
    Address,
    callbacks::{ReadMemory, WriteMemory},
};
use address_space::{AddressSpace, AddressSpaceHandle};
use bitvec::{field::BitField, order::Lsb0};
use rangemap::RangeInclusiveMap;
use std::{
    fmt::Debug,
    num::NonZero,
    ops::RangeInclusive,
    sync::{
        RwLock,
        atomic::{AtomicU8, Ordering},
    },
};
use store::MemoryStore;

pub mod address_space;
mod read;
mod store;
mod write;

pub use read::*;
pub use write::*;

/// For the initial access
const NEEDED_ACCESSES_BASE_CAPACITY: usize = 1;

pub struct RemapCallback {
    callback: Box<dyn FnOnce(&MemoryTranslationTable)>,
}

impl RemapCallback {
    pub fn new(callback: impl FnOnce(&MemoryTranslationTable) + 'static) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }
}

#[allow(clippy::type_complexity)]
pub struct MemoryOperationError<R> {
    /// Records the memory translation table should handle
    pub records: RangeInclusiveMap<Address, R>,
    /// Allows remapping of the MTT when its safe. The semantics of when this occurs is unspecified except that the caller that triggered this will not return until the remap(s) occurs.
    pub remap_callback: Option<RemapCallback>,
}

impl<R> From<RangeInclusiveMap<Address, R>> for MemoryOperationError<R> {
    fn from(records: RangeInclusiveMap<Address, R>) -> Self {
        Self {
            records,
            remap_callback: None,
        }
    }
}

trait ReadWriteMemory: ReadMemory + WriteMemory {}
impl<M: ReadMemory + WriteMemory> ReadWriteMemory for M {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct MemoryHandle(NonZero<u16>);

impl MemoryHandle {
    pub(crate) const fn get(&self) -> usize {
        (self.0.get() as usize) - 1
    }
}

impl MemoryHandle {
    pub(crate) const fn new(id: NonZero<u16>) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub struct MemoryTranslationTable {
    address_spaces: RwLock<Vec<AddressSpace>>,
    memory_store: MemoryStore,
    current_address_space: AtomicU8,
}

impl Default for MemoryTranslationTable {
    fn default() -> Self {
        Self {
            address_spaces: Default::default(),
            memory_store: Default::default(),
            current_address_space: AtomicU8::new(1),
        }
    }
}

impl MemoryTranslationTable {
    pub(crate) fn insert_address_space(&self, address_space_width: u8) -> AddressSpaceHandle {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();

        let id = self.current_address_space.fetch_add(1, Ordering::Relaxed);
        let id = AddressSpaceHandle::new(id.try_into().expect("Too many address spaces"));

        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load();

        address_spaces_guard.push(AddressSpace {
            width_mask,
            read_members: RangeInclusiveMap::new(),
            write_members: RangeInclusiveMap::new(),
        });

        id
    }

    pub fn address_spaces(&self) -> impl Iterator<Item = AddressSpaceHandle> {
        let address_space_count = self.address_spaces.read().unwrap().len();

        (0..address_space_count)
            .map(|id| AddressSpaceHandle::new(NonZero::new((id + 1) as u8).unwrap()))
    }

    pub fn remap_memory(
        &self,
        handle: MemoryHandle,
        address_space: AddressSpaceHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();
        let address_space = address_spaces_guard.get_mut(address_space.get()).unwrap();

        assert!(
            self.memory_store.is_readwrite_memory(handle),
            "Memory referred by handle does not have read & write capabilities"
        );

        address_space.remap_memory(handle, mapping);
    }

    pub(crate) fn insert_memory<M: ReadMemory + WriteMemory>(&self, memory: M) -> MemoryHandle {
        self.memory_store.insert_memory(memory)
    }

    #[inline]
    fn process_remap_callbacks(&self, callbacks: impl IntoIterator<Item = RemapCallback>) {
        for callback in callbacks {
            (callback.callback)(self);
        }
    }
}

struct NeededAccess {
    pub address: Address,
    pub address_space: AddressSpaceHandle,
    pub buffer_subrange: RangeInclusive<Address>,
}
