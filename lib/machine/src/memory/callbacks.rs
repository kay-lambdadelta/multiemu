use super::{
    AddressSpaceId,
    memory_translation_table::{PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord},
};
use rangemap::RangeInclusiveMap;

pub trait ReadMemory: Send + Sync + 'static {
    fn read_memory(
        &self,
        address: usize,
        buffer: &mut [u8],
        address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    );
}

impl<
    F: Fn(usize, &mut [u8], AddressSpaceId, &mut RangeInclusiveMap<usize, ReadMemoryRecord>)
        + Send
        + Sync
        + 'static,
> ReadMemory for F
{
    fn read_memory(
        &self,
        address: usize,
        buffer: &mut [u8],
        address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        self(address, buffer, address_space, errors);
    }
}

pub trait WriteMemory: Send + Sync + 'static {
    fn write_memory(
        &self,
        address: usize,
        buffer: &[u8],
        address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    );
}

impl<
    F: Fn(usize, &[u8], AddressSpaceId, &mut RangeInclusiveMap<usize, WriteMemoryRecord>)
        + Send
        + Sync
        + 'static,
> WriteMemory for F
{
    fn write_memory(
        &self,
        address: usize,
        buffer: &[u8],
        address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        self(address, buffer, address_space, errors);
    }
}

pub trait PreviewMemory: Send + Sync + 'static {
    fn preview_memory(
        &self,
        address: usize,
        buffer: &mut [u8],
        address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    );
}

impl<
    F: Fn(usize, &mut [u8], AddressSpaceId, &mut RangeInclusiveMap<usize, PreviewMemoryRecord>)
        + Send
        + Sync
        + 'static,
> PreviewMemory for F
{
    fn preview_memory(
        &self,
        address: usize,
        buffer: &mut [u8],
        address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    ) {
        self(address, buffer, address_space, errors);
    }
}
