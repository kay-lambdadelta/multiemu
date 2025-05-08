use std::fmt::Debug;

use multiemu_machine::memory::{
    AddressSpaceHandle, callbacks::ReadMemory, memory_translation_table::ReadMemoryRecord,
};
use rangemap::RangeInclusiveMap;

pub struct RawCartridgeMemoryCallback {
    pub rom: [u8; 0x1000],
}

impl Debug for RawCartridgeMemoryCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawCartridgeMemoryCallback")
            .field("rom", &"[..]")
            .finish()
    }
}

impl ReadMemory for RawCartridgeMemoryCallback {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        _errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        let adjusted_offset = address - 0x1000;
        buffer.copy_from_slice(&self.rom[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);
    }
}
