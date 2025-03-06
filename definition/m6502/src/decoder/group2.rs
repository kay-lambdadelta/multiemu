use super::ARGUMENT;
use crate::instruction::{AddressingMode, M6502InstructionSet, M6502InstructionSetSpecifier};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};
use multiemu_machine::memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable};

#[inline]
pub fn decode_group2_space_instruction(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (M6502InstructionSet, u8) {
    let argument = instruction_first_byte[ARGUMENT].load::<u8>();

    // Any case of ZeroPageIndirectYIndexed here is JAM
    if (
        argument == 0b100
    ) // A few cases of XIndexedZeroPageIndirect here are JAM
     || ([0b000, 0b001, 0b010, 0b011].contains(&instruction_identifier)
        && (
            argument ==
            0b000
        ))
    {
        return (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Jam,
                addressing_mode: None,
            },
            0,
        );
    }

    match instruction_identifier {
        0b000 => {
            if argument == 0b110 {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    0,
                )
            } else {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Asl,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b001 => {
            if argument == 0b110 {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    0,
                )
            } else {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Rol,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b010 => {
            if argument == 0b110 {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    0,
                )
            } else {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Lsr,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b011 => {
            if argument == 0b110 {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    0,
                )
            } else {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Ror,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
        }
        0b100 => match argument {
            0b000 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut immediate),
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(AddressingMode::Immediate(immediate)),
                    },
                    1,
                )
            }
            0b001 | 0b011 | 0b101 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Stx,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Txa,
                    addressing_mode: None,
                },
                0,
            ),
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Txs,
                    addressing_mode: None,
                },
                0,
            ),
            0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Shx,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            _ => unreachable!(),
        },
        0b101 => match argument {
            0b000 | 0b001 | 0b011 | 0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Ldx,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Tax,
                    addressing_mode: None,
                },
                0,
            ),
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Tsx,
                    addressing_mode: None,
                },
                0,
            ),
            _ => unreachable!(),
        },
        0b110 => match argument {
            0b000 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut immediate),
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(AddressingMode::Immediate(immediate)),
                    },
                    1,
                )
            }
            0b001 | 0b011 | 0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Dec,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Dex,
                    addressing_mode: None,
                },
                0,
            ),
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Nop,
                    addressing_mode: None,
                },
                0,
            ),
            _ => unreachable!(),
        },
        0b111 => match argument {
            0b000 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut immediate),
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(AddressingMode::Immediate(immediate)),
                    },
                    1,
                )
            }
            0b001 | 0b011 | 0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group2_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Inc,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 | 0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Nop,
                    addressing_mode: None,
                },
                0,
            ),
            _ => unreachable!(),
        },
        _ => {
            unreachable!()
        }
    }
}
