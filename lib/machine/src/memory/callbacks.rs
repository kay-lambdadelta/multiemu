use super::memory_translation_table::{
    MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord,
    address_space::AddressSpaceHandle,
};
use rangemap::RangeInclusiveMap;
use std::{fmt::Debug, sync::Arc};

#[allow(unused)]
pub trait ReadMemory: Debug + Send + Sync + 'static {
    fn read_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        Err(RangeInclusiveMap::from_iter([(
            address..=(address + (buffer.len() - 1)),
            ReadMemoryRecord::Denied,
        )])
        .into())
    }

    fn preview_memory(
        &self,
        address: usize,
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
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        self.as_ref().read_memory(address, address_space, buffer)
    }

    fn preview_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        self.as_ref().preview_memory(address, address_space, buffer)
    }
}

#[allow(unused)]
pub trait WriteMemory: Debug + Send + Sync + 'static {
    fn write_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        Err(RangeInclusiveMap::from_iter([(
            address..=(address + (buffer.len() - 1)),
            WriteMemoryRecord::Denied,
        )])
        .into())
    }
}

impl<M: WriteMemory> WriteMemory for Arc<M> {
    fn write_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        self.as_ref().write_memory(address, address_space, buffer)
    }
}
