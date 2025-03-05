use crate::memory::AddressSpaceId;
use crate::memory::memory_translation_table::MemoryTranslationTable;
use crate::processor::instruction::InstructionSet;
use std::fmt::Debug;

pub trait InstructionDecoder: Debug + Send + Sync + 'static {
    type InstructionSet: InstructionSet;

    fn decode(
        &self,
        cursor: usize,
        address_space: AddressSpaceId,
        memory_translation_table: &MemoryTranslationTable,
    ) -> Option<(Self::InstructionSet, u8)>;
}
