use multiemu_machine::memory::{
    AddressSpaceId, callbacks::Memory, memory_translation_table::ReadMemoryRecord,
};
use rangemap::RangeInclusiveMap;

#[derive(Debug)]
pub struct RawCartridgeMemoryCallback {
    pub rom: Vec<u8>,
}

impl Memory for RawCartridgeMemoryCallback {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
        _errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        let adjusted_offset = address - 0xf000;
        buffer.copy_from_slice(&self.rom[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);
    }
}
