use crate::{
    memory::memory_translation_table::{MemoryTranslationTable, address_space::AddressSpaceHandle},
    processor::instruction::InstructionSet,
};
use std::fmt::Debug;

pub trait InstructionDecoder: Debug + Send + Sync + 'static {
    type InstructionSet: InstructionSet;

    fn decode(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        memory_translation_table: &MemoryTranslationTable,
    ) -> Option<(Self::InstructionSet, u8)>;
}
