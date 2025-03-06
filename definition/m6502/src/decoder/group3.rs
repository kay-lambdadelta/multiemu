use super::ARGUMENT;
use crate::instruction::{AddressingMode, M6502InstructionSet, M6502InstructionSetSpecifier};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};
use multiemu_machine::memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable};

// This one is hellish to decode

#[inline]
pub fn decode_group3_space_instruction(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (M6502InstructionSet, u8) {
    let argument = instruction_first_byte[ARGUMENT].load::<u8>();

    match instruction_identifier {
        0b000 => match argument {
            0b000 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Brk,
                    addressing_mode: None,
                },
                0,
            ),
            0b001 | 0b011 | 0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Php,
                    addressing_mode: None,
                },
                0,
            ),
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bpl,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Clc,
                    addressing_mode: None,
                },
                0,
            ),
            _ => {
                unreachable!()
            }
        },
        0b001 => match argument {
            0b000 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                let absolute = u16::from_le_bytes(absolute);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Jsr,
                        addressing_mode: Some(AddressingMode::Absolute(absolute)),
                    },
                    2,
                )
            }
            0b001 | 0b011 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bit,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Plp,
                    addressing_mode: None,
                },
                0,
            ),
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bmi,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sec,
                    addressing_mode: None,
                },
                0,
            ),
            _ => {
                unreachable!()
            }
        },
        0b010 => match argument {
            0b000 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Rti,
                    addressing_mode: None,
                },
                0,
            ),
            0b001 | 0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Pha,
                    addressing_mode: None,
                },
                0,
            ),
            0b011 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Jmp,
                        addressing_mode: Some(AddressingMode::Absolute(u16::from_le_bytes(
                            absolute,
                        ))),
                    },
                    2,
                )
            }
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bvc,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Cli,
                    addressing_mode: None,
                },
                0,
            ),
            _ => {
                unreachable!()
            }
        },
        0b011 => match argument {
            0b000 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Rts,
                    addressing_mode: None,
                },
                0,
            ),
            0b001 | 0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Pla,
                    addressing_mode: None,
                },
                0,
            ),
            0b011 => {
                let mut absolute_indirect = [0; 2];
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    &mut absolute_indirect,
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Jmp,
                        addressing_mode: Some(AddressingMode::AbsoluteIndirect(
                            u16::from_le_bytes(absolute_indirect),
                        )),
                    },
                    2,
                )
            }
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bvs,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sei,
                    addressing_mode: None,
                },
                0,
            ),
            _ => {
                unreachable!()
            }
        },
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
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Sty,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Dey,
                    addressing_mode: None,
                },
                0,
            ),
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bcc,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Tya,
                    addressing_mode: None,
                },
                0,
            ),
            0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Shy,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            _ => {
                unreachable!()
            }
        },
        0b101 => match argument {
            0b000 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut immediate),
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Ldy,
                        addressing_mode: Some(AddressingMode::Immediate(immediate)),
                    },
                    1,
                )
            }
            0b001 | 0b011 | 0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Ldy,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Tay,
                    addressing_mode: None,
                },
                0,
            ),
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bcs,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Clv,
                    addressing_mode: None,
                },
                0,
            ),
            _ => {
                unreachable!()
            }
        },
        0b110 => match argument {
            0b000 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    // Cast a i8 as a &mut [u8; 1]
                    std::array::from_mut(&mut immediate),
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Cpy,
                        addressing_mode: Some(AddressingMode::Immediate(immediate)),
                    },
                    1,
                )
            }
            0b001 | 0b011 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Cpy,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Iny,
                    addressing_mode: None,
                },
                0,
            ),
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bne,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Cld,
                    addressing_mode: None,
                },
                0,
            ),
            _ => {
                unreachable!()
            }
        },
        0b111 => match argument {
            0b000 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    // Cast a i8 as a &mut [u8; 1]
                    std::array::from_mut(&mut immediate),
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Cpx,
                        addressing_mode: Some(AddressingMode::Immediate(immediate)),
                    },
                    1,
                )
            }
            0b001 | 0b011 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Cpx,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Inx,
                    addressing_mode: None,
                },
                0,
            ),
            0b100 => {
                let relative =
                    load_relative_address_modifier(cursor, address_space, memory_translation_table);

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Beq,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    1,
                )
            }
            0b101 | 0b111 => {
                let (addressing_mode, added_instruction_length) =
                    AddressingMode::from_group1_addressing(
                        cursor,
                        address_space,
                        memory_translation_table,
                        argument,
                    );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    added_instruction_length,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sed,
                    addressing_mode: None,
                },
                0,
            ),
            _ => {
                unreachable!()
            }
        },
        _ => {
            unreachable!()
        }
    }
}

#[inline]
fn load_relative_address_modifier(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
) -> i8 {
    let mut relative = 0;
    let _ = memory_translation_table.read(
        cursor as usize,
        address_space,
        // Cast a i8 as a &mut [u8; 1]
        bytemuck::bytes_of_mut(&mut relative),
    );
    relative
}
