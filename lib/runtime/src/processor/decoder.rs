use std::fmt::Debug;

use crate::{
    memory::{Address, AddressSpace, AddressSpaceCache},
    processor::instruction::InstructionSet,
};

/// Represented a decoder for instructions
pub trait InstructionDecoder: Debug + Send + Sync + 'static {
    /// the instruction set this type uses
    type InstructionSet: InstructionSet;

    /// The decoder
    fn decode(
        &self,
        address: Address,
        address_space: &AddressSpace,
        address_space_cache: Option<&mut AddressSpaceCache>,
    ) -> Option<(Self::InstructionSet, u8)>;
}
