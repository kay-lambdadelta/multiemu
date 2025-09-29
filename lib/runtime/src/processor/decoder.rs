use crate::{
    memory::{Address, AddressSpaceId, MemoryAccessTable},
    processor::instruction::InstructionSet,
};
use std::fmt::Debug;

/// Represented a decoder for instructions
pub trait InstructionDecoder: Debug + Send + Sync + 'static {
    /// the instruction set this type uses
    type InstructionSet: InstructionSet;

    /// The decoder
    fn decode(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        memory_access_table: &MemoryAccessTable,
    ) -> Option<(Self::InstructionSet, u8)>;
}
