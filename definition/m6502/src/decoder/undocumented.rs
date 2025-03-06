use super::ARGUMENT;
use crate::instruction::{AddressingMode, M6502InstructionSet, M6502InstructionSetSpecifier};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};
use multiemu_machine::memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable};

#[inline]
pub fn decode_undocumented_space_instruction(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (M6502InstructionSet, u8) {
    let addressing_mode_byte = instruction_first_byte[ARGUMENT].load::<u8>();

    let (addressing_mode, added_instruction_length) = AddressingMode::from_group1_addressing(
        cursor,
        address_space,
        memory_translation_table,
        addressing_mode_byte,
    );

    match instruction_identifier {
        0b000 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Anc,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Slo,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b001 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Anc,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Rla,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b010 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Anc,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Sre,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b011 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Anc,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Rra,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b100 => match addressing_mode {
            AddressingMode::XIndexedZeroPageIndirect(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sax,
                    addressing_mode: Some(addressing_mode),
                },
                added_instruction_length,
            ),
            AddressingMode::Immediate(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Xaa,
                    addressing_mode: Some(addressing_mode),
                },
                added_instruction_length,
            ),
            AddressingMode::Absolute(..) | AddressingMode::ZeroPage(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sax,
                    addressing_mode: Some(addressing_mode),
                },
                added_instruction_length,
            ),
            AddressingMode::XIndexedZeroPage(value) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sax,
                    addressing_mode: Some(AddressingMode::YIndexedZeroPage(value)),
                },
                added_instruction_length,
            ),
            AddressingMode::YIndexedAbsolute(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Shs,
                    addressing_mode: Some(addressing_mode),
                },
                added_instruction_length,
            ),
            AddressingMode::XIndexedAbsolute(address) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sha,
                    addressing_mode: Some(AddressingMode::YIndexedAbsolute(address)),
                },
                added_instruction_length,
            ),
            AddressingMode::ZeroPageIndirectYIndexed(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sha,
                    addressing_mode: Some(addressing_mode),
                },
                added_instruction_length,
            ),
            _ => unreachable!(),
        },
        0b101 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Las,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Lax,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b110 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Sbx,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Dcp,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b111 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Sbc,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Isc,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        _ => {
            unreachable!()
        }
    }
}
