use super::{
    Address,
    memory_translation_table::{
        MemoryHandle, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
        WriteMemoryRecord, address_space::AddressSpaceHandle,
    },
};
use std::{fmt::Debug, sync::Arc};

#[allow(unused)]
pub trait Memory: Debug + Send + Sync + 'static {
    fn set_memory_handle(&self, handle: MemoryHandle) {}
}

impl<M: Memory> Memory for Arc<M> {
    fn set_memory_handle(&self, handle: MemoryHandle) {
        self.as_ref().set_memory_handle(handle)
    }
}

#[allow(unused)]
pub trait ReadMemory: Memory {
    fn read_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>>;

    fn preview_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        // Convert between a read and a preview

        self.read_memory(address, address_space, buffer)
            .map_err(|e| MemoryOperationError {
                records: e
                    .records
                    .into_iter()
                    .map(|(range, record)| {
                        (
                            range,
                            match record {
                                ReadMemoryRecord::Denied => PreviewMemoryRecord::Denied,
                                ReadMemoryRecord::Redirect {
                                    address,
                                    address_space,
                                } => PreviewMemoryRecord::Redirect {
                                    address,
                                    address_space,
                                },
                            },
                        )
                    })
                    .collect(),
                remap_callback: e.remap_callback,
            })
    }
}

impl<M: ReadMemory> ReadMemory for Arc<M> {
    fn read_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        self.as_ref().read_memory(address, address_space, buffer)
    }

    fn preview_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        self.as_ref().preview_memory(address, address_space, buffer)
    }
}

#[allow(unused)]
pub trait WriteMemory: Memory {
    fn write_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>>;
}

impl<M: WriteMemory> WriteMemory for Arc<M> {
    fn write_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        self.as_ref().write_memory(address, address_space, buffer)
    }
}
