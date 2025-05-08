use super::{
    AddressSpaceHandle,
    memory_translation_table::{PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord},
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
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        errors.insert(
            address..=(address + (buffer.len() - 1)),
            ReadMemoryRecord::Denied,
        );
    }

    fn preview_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    ) {
        errors.insert(
            address..=(address + (buffer.len() - 1)),
            PreviewMemoryRecord::Impossible,
        );
    }
}

impl<M: ReadMemory> ReadMemory for Arc<M> {
    fn read_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        self.as_ref()
            .read_memory(address, address_space, buffer, errors);
    }

    fn preview_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    ) {
        self.as_ref()
            .preview_memory(address, address_space, buffer, errors);
    }
}

#[allow(unused)]
pub trait WriteMemory: Debug + Send + Sync + 'static {
    fn write_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        errors.insert(
            address..=(address + (buffer.len() - 1)),
            WriteMemoryRecord::Denied,
        );
    }
}

impl<M: WriteMemory> WriteMemory for Arc<M> {
    fn write_memory(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        self.as_ref()
            .write_memory(address, address_space, buffer, errors);
    }
}
