use multiemu_machine::memory::{
    Address,
    callbacks::{Memory, ReadMemory},
    memory_translation_table::{
        MemoryOperationError, ReadMemoryRecord, address_space::AddressSpaceHandle,
    },
};

#[derive(Debug)]
pub struct BankingCartridgeMemoryCallback<const BANK_SIZE: usize> {
    rom: Vec<u8>,
}

impl<const BANK_SIZE: usize> Memory for BankingCartridgeMemoryCallback<BANK_SIZE> {}

impl<const BANK_SIZE: usize> BankingCartridgeMemoryCallback<BANK_SIZE> {
    pub fn new(rom: Vec<u8>) -> Self {
        assert!(
            rom.len() % BANK_SIZE == 0,
            "ROM size must be a multiple of BANK_SIZE"
        );
        Self { rom }
    }
}

impl<const BANK_SIZE: usize> ReadMemory for BankingCartridgeMemoryCallback<BANK_SIZE> {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let adjusted_offset = address - 0x1000;
        buffer.copy_from_slice(&self.rom[adjusted_offset..=(adjusted_offset + (buffer.len() - 1))]);

        Ok(())
    }
}
