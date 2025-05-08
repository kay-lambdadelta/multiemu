use super::instruction::{AddressingMode, Mos6502InstructionSet};
use bitvec::{field::BitField, prelude::Msb0, view::BitView};
use group1::decode_group1_space_instruction;
use group2::decode_group2_space_instruction;
use group3::decode_group3_space_instruction;
use multiemu_machine::{
    memory::{AddressSpaceHandle, memory_translation_table::MemoryTranslationTable},
    processor::decoder::InstructionDecoder,
};
use std::ops::Range;
use strum::FromRepr;
use undocumented::decode_undocumented_space_instruction;

mod group1;
mod group2;
mod group3;
mod undocumented;

// https://www.pagetable.com/c64ref/6502/

#[derive(Debug, Default)]
pub struct Mos6502InstructionDecoder;

impl InstructionDecoder for Mos6502InstructionDecoder {
    type InstructionSet = Mos6502InstructionSet;

    fn decode(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        memory_translation_table: &MemoryTranslationTable,
    ) -> Option<(Self::InstructionSet, u8)> {
        let byte: u8 = memory_translation_table
            .read_le_value(address, address_space)
            .unwrap_or_default();

        let byte = byte.view_bits::<Msb0>();
        let instruction_identifier =
            InstructionGroup::from_repr(byte[INSTRUCTION_IDENTIFIER].load::<u8>()).unwrap();
        let secondary_instruction_identifier = byte[SECONDARY_INSTRUCTION_IDENTIFIER].load::<u8>();

        let result = Some(match instruction_identifier {
            InstructionGroup::Group3 => {
                decode_group3_space_instruction(secondary_instruction_identifier, byte)
            }
            InstructionGroup::Group1 => {
                decode_group1_space_instruction(secondary_instruction_identifier, byte)
            }
            InstructionGroup::Group2 => {
                decode_group2_space_instruction(secondary_instruction_identifier, byte)
            }
            InstructionGroup::Undocumented => {
                decode_undocumented_space_instruction(secondary_instruction_identifier, byte)
            }
        });

        result.map(|(opcode, addressing_mode)| {
            (
                Mos6502InstructionSet {
                    opcode,
                    addressing_mode,
                },
                1,
            )
        })
    }
}

const INSTRUCTION_IDENTIFIER: Range<usize> = 6..8;
const SECONDARY_INSTRUCTION_IDENTIFIER: Range<usize> = 0..3;
const ARGUMENT: Range<usize> = 3..6;

#[derive(FromRepr)]
#[repr(u8)]
enum InstructionGroup {
    Group3 = 0b00,
    Group1 = 0b01,
    Group2 = 0b10,
    Undocumented = 0b11,
}

impl AddressingMode {
    // Really need this to be inlined
    #[inline(always)]
    pub fn from_group1_addressing(addressing_mode: u8) -> Self {
        match addressing_mode {
            0b000 => AddressingMode::XIndexedZeroPageIndirect,
            0b001 => AddressingMode::ZeroPage,
            0b010 => AddressingMode::Immediate,
            0b011 => AddressingMode::Absolute,
            0b100 => AddressingMode::ZeroPageIndirectYIndexed,
            0b101 => AddressingMode::XIndexedZeroPage,
            0b110 => AddressingMode::YIndexedAbsolute,
            0b111 => AddressingMode::XIndexedAbsolute,
            _ => {
                unreachable!()
            }
        }
    }

    #[inline]
    pub fn from_group2_addressing(addressing_mode: u8) -> Self {
        match addressing_mode {
            0b000 => AddressingMode::Immediate,
            0b001 => AddressingMode::ZeroPage,
            0b010 => AddressingMode::Accumulator,
            0b011 => AddressingMode::Absolute,
            0b100 => AddressingMode::ZeroPageIndirectYIndexed,
            0b101 => AddressingMode::XIndexedZeroPage,
            0b110 => AddressingMode::YIndexedAbsolute,
            0b111 => AddressingMode::XIndexedAbsolute,
            _ => {
                unreachable!()
            }
        }
    }
}
