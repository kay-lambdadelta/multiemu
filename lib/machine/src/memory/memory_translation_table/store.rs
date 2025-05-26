use super::{MemoryHandle, ReadWriteMemory};
use crate::memory::callbacks::{ReadMemory, WriteMemory};
use std::sync::{
    RwLock,
    atomic::{AtomicU16, Ordering},
};

#[derive(Debug)]
enum StoredCallback {
    Read(Box<dyn ReadMemory>),
    Write(Box<dyn WriteMemory>),
    ReadWrite(Box<dyn ReadWriteMemory>),
}

#[derive(Debug)]
pub(crate) struct MemoryStore {
    current_memory_handle: AtomicU16,
    store: RwLock<Vec<StoredCallback>>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            current_memory_handle: AtomicU16::new(1),
            store: RwLock::default(),
        }
    }
}

impl MemoryStore {
    pub fn insert_read_memory<M: ReadMemory>(&self, memory: M) -> MemoryHandle {
        let mut store = self.store.write().unwrap();
        let handle = self.allocate_handle();

        store.push(StoredCallback::Read(Box::new(memory)));

        handle
    }

    pub fn insert_write_memory<M: WriteMemory>(&self, memory: M) -> MemoryHandle {
        let mut store = self.store.write().unwrap();
        let handle = self.allocate_handle();

        store.push(StoredCallback::Write(Box::new(memory)));

        handle
    }

    pub fn insert_memory<M: ReadWriteMemory>(&self, memory: M) -> MemoryHandle {
        let mut store = self.store.write().unwrap();
        let handle = self.allocate_handle();

        store.push(StoredCallback::ReadWrite(Box::new(memory)));

        handle
    }

    pub fn is_read_memory(&self, handle: MemoryHandle) -> bool {
        return matches!(
            self.store.read().unwrap().get(handle.get()),
            Some(StoredCallback::Read(_)) | Some(StoredCallback::ReadWrite(_))
        );
    }

    pub fn is_write_memory(&self, handle: MemoryHandle) -> bool {
        return matches!(
            self.store.read().unwrap().get(handle.get()),
            Some(StoredCallback::Write(_)) | Some(StoredCallback::ReadWrite(_))
        );
    }

    pub fn is_readwrite_memory(&self, handle: MemoryHandle) -> bool {
        return matches!(
            self.store.read().unwrap().get(handle.get()),
            Some(StoredCallback::ReadWrite(_))
        );
    }

    #[inline]
    pub fn interact_read<T>(
        &self,
        handle: MemoryHandle,
        mut callback: impl FnMut(&dyn ReadMemory) -> T,
    ) -> T {
        match self.store.read().unwrap().get(handle.get()) {
            Some(StoredCallback::Read(memory)) => callback(&**memory),
            Some(StoredCallback::ReadWrite(memory)) => callback(&**memory),
            _ => panic!("Memory referred by handle does not have read capabilities"),
        }
    }

    #[inline]
    pub fn interact_write<T>(
        &self,
        handle: MemoryHandle,
        mut callback: impl FnMut(&dyn WriteMemory) -> T,
    ) -> T {
        match self.store.read().unwrap().get(handle.get()) {
            Some(StoredCallback::Write(memory)) => callback(&**memory),
            Some(StoredCallback::ReadWrite(memory)) => callback(&**memory),
            c => panic!("Memory referred by handle does not have write capabilities {:?}", c),
        }
    }

    fn allocate_handle(&self) -> MemoryHandle {
        let handle = self.current_memory_handle.fetch_add(1, Ordering::Relaxed);
        // If its zero, then we have rolled over and its time to complain!

        MemoryHandle::new(handle.try_into().expect("Too many address spaces"))
    }
}
