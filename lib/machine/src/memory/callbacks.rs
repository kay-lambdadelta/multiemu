use super::{
    AddressSpaceHandle,
    memory_translation_table::{PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord},
};
use rangemap::RangeInclusiveMap;
use std::fmt::Debug;

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
