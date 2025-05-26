use multiemu_machine::memory::{
    callbacks::ReadMemory,
    memory_translation_table::{
        MemoryOperationError, ReadMemoryRecord, address_space::AddressSpaceHandle,
    },
};
use std::fmt::Debug;

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
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let adjusted_offset = address - 0x1000;
        buffer.copy_from_slice(&self.rom[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);

        Ok(())
    }
}
