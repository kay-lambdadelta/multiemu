use super::Address;
use crate::component::{ComponentId, ComponentRegistry};
use address_space::AddressSpace;
use bitvec::{field::BitField, order::Lsb0};
use nohash::BuildNoHashHasher;
use rangemap::RangeInclusiveMap;
use std::{
    boxed::Box,
    collections::HashMap,
    ops::RangeInclusive,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU16, Ordering},
    },
};

mod address_space;
mod read;
mod write;

pub use address_space::AddressSpaceHandle;
pub use read::*;
pub use write::*;

/// Callback to be able to remap the memory translation table without a deadlock
pub struct RemapCallback {
    callback: Box<dyn FnOnce(&MemoryAccessTable) + Send>,
}

impl RemapCallback {
    /// Create a new remap callback from a closure
    pub fn new(callback: impl FnOnce(&MemoryAccessTable) + Send + 'static) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }
}

#[allow(clippy::type_complexity)]
/// Error type from componenents
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
#[derive(Debug)]
/// The main structure representing the devices memory address spaces
pub struct MemoryAccessTable {
    address_spaces: RwLock<HashMap<AddressSpaceHandle, AddressSpace, BuildNoHashHasher<u16>>>,
    current_address_space: AtomicU16,
    component_store: Arc<ComponentRegistry>,
}

impl MemoryAccessTable {
    pub(crate) fn new(component_store: Arc<ComponentRegistry>) -> Self {
        Self {
            address_spaces: Default::default(),
            current_address_space: AtomicU16::new(1),
            component_store,
        }
    }

    pub(crate) fn insert_address_space(&self, address_space_width: u8) -> AddressSpaceHandle {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();

        let id =
            AddressSpaceHandle::new(self.current_address_space.fetch_add(1, Ordering::Relaxed))
                .unwrap();

        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load();

        address_spaces_guard.insert(id, AddressSpace::new(width_mask));

        id
    }

    /// Iter over present spaces
    pub fn address_spaces(&self) -> impl Iterator<Item = AddressSpaceHandle> {
        let address_space_count = self.address_spaces.read().unwrap().len();

        (0..address_space_count).map(|id| AddressSpaceHandle::new((id + 1) as u16).unwrap())
    }

    /// Remap memory in a specific address space, clearing previous mappings
    pub fn remap_memory(
        &self,
        component_id: ComponentId,
        address_space: AddressSpaceHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();
        let address_space = address_spaces_guard.get_mut(&address_space).unwrap();

        address_space.remap_memory(component_id, mapping);
    }

    #[inline]
    fn process_remap_callbacks(&self, callbacks: impl IntoIterator<Item = RemapCallback>) {
        for callback in callbacks {
            (callback.callback)(self);
        }
    }
}
