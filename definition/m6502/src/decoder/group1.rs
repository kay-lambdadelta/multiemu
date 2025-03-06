use super::ARGUMENT;
use crate::instruction::{AddressingMode, M6502InstructionSet, M6502InstructionSetSpecifier};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};
use multiemu_machine::memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable};

#[inline]
pub fn decode_group1_space_instruction(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (M6502InstructionSet, u8) {
    let addressing_mode = instruction_first_byte[ARGUMENT].load::<u8>();

    let (addressing_mode, added_instruction_length) = AddressingMode::from_group1_addressing(
        cursor,
        address_space,
        memory_translation_table,
        addressing_mode,
    );

    match instruction_identifier {
        0b000 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Ora,
                addressing_mode: Some(addressing_mode),
            },
            added_instruction_length,
        ),
        0b001 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::And,
                addressing_mode: Some(addressing_mode),
            },
            added_instruction_length,
        ),
        0b010 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Eor,
                addressing_mode: Some(addressing_mode),
            },
            added_instruction_length,
        ),
        0b011 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Adc,
                addressing_mode: Some(addressing_mode),
            },
            added_instruction_length,
        ),
        0b100 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                // STA immediate is NOP
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Sta,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b101 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Lda,
                addressing_mode: Some(addressing_mode),
            },
            added_instruction_length,
        ),
        0b110 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Cmp,
                addressing_mode: Some(addressing_mode),
            },
            added_instruction_length,
        ),
        0b111 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Sbc,
                addressing_mode: Some(addressing_mode),
            },
            added_instruction_length,
        ),
        _ => {
            unreachable!()
        }
    }
}
