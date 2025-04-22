use crate::{
    memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable},
    processor::instruction::InstructionSet,
};
use std::fmt::Debug;

pub trait InstructionDecoder: Debug + Send + Sync + 'static {
    type InstructionSet: InstructionSet;

    fn decode(
        &self,
        address: usize,
        address_space: AddressSpaceId,
        memory_translation_table: &MemoryTranslationTable,
    ) -> Option<(Self::InstructionSet, u8)>;
}
