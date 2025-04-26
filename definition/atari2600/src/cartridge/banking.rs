use multiemu_machine::memory::{
    AddressSpaceId, callbacks::Memory, memory_translation_table::ReadMemoryRecord,
};
use rangemap::RangeInclusiveMap;

#[derive(Debug)]
pub struct BankingCartridgeMemoryCallback<const BANK_SIZE: usize> {
    rom: Vec<u8>,
}

impl<const BANK_SIZE: usize> BankingCartridgeMemoryCallback<BANK_SIZE> {
    pub fn new(rom: Vec<u8>) -> Self {
        assert!(
            rom.len() % BANK_SIZE == 0,
            "ROM size must be a multiple of BANK_SIZE"
        );
        Self { rom }
    }
}

impl<const BANK_SIZE: usize> Memory for BankingCartridgeMemoryCallback<BANK_SIZE> {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
        _errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        let adjusted_offset = address - 0x1000;
        buffer.copy_from_slice(&self.rom[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);
    }
}
