use super::{MemoryHandle, ReadWriteMemory};
use crate::memory::callbacks::{ReadMemory, WriteMemory};
use std::{
    boxed::Box,
    sync::{
        RwLock,
        atomic::{AtomicU16, Ordering},
    },
    vec::Vec,
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

        memory.set_memory_handle(handle);
        store.push(StoredCallback::Read(Box::new(memory)));

        handle
    }

    pub fn insert_write_memory<M: WriteMemory>(&self, memory: M) -> MemoryHandle {
        let mut store = self.store.write().unwrap();
        let handle = self.allocate_handle();

        memory.set_memory_handle(handle);
        store.push(StoredCallback::Write(Box::new(memory)));

        handle
    }

    pub fn insert_memory<M: ReadWriteMemory>(&self, memory: M) -> MemoryHandle {
        let mut store = self.store.write().unwrap();
        let handle = self.allocate_handle();

        memory.set_memory_handle(handle);
        store.push(StoredCallback::ReadWrite(Box::new(memory)));

        handle
    }

    #[inline]
    pub fn is_read_memory(&self, handle: MemoryHandle) -> bool {
        return matches!(
            self.store.read().unwrap().get(handle.get()),
            Some(StoredCallback::Read(_)) | Some(StoredCallback::ReadWrite(_))
        );
    }

    #[inline]
    pub fn is_write_memory(&self, handle: MemoryHandle) -> bool {
        return matches!(
            self.store.read().unwrap().get(handle.get()),
            Some(StoredCallback::Write(_)) | Some(StoredCallback::ReadWrite(_))
        );
    }

    #[inline]
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
        let store_guard = self.store.read().unwrap();
        let memory = store_guard
            .get(handle.get())
            .expect("Could not find memory");

        match memory {
            StoredCallback::Read(memory) => callback(&**memory),
            StoredCallback::ReadWrite(memory) => callback(&**memory),
            StoredCallback::Write(memory) => {
                panic!(
                    "Memory referred by handle does not have read capabilities: {:?}",
                    memory
                )
            }
        }
    }

    #[inline]
    pub fn interact_write<T>(
        &self,
        handle: MemoryHandle,
        mut callback: impl FnMut(&dyn WriteMemory) -> T,
    ) -> T {
        let store_guard = self.store.read().unwrap();
        let memory = store_guard
            .get(handle.get())
            .expect("Could not find memory");

        match memory {
            StoredCallback::Write(memory) => callback(&**memory),
            StoredCallback::ReadWrite(memory) => callback(&**memory),
            StoredCallback::Read(memory) => {
                panic!(
                    "Memory referred by handle does not have write capabilities: {:?}",
                    memory
                )
            }
        }
    }

    fn allocate_handle(&self) -> MemoryHandle {
        let handle = self.current_memory_handle.fetch_add(1, Ordering::Relaxed);
        // If its zero, then we have rolled over and its time to complain!

        MemoryHandle::new(handle.try_into().expect("Too many address spaces"))
    }
}
