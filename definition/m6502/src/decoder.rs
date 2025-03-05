use super::instruction::{AddressingMode, M6502InstructionSet, M6502InstructionSetSpecifier};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
    view::BitView,
};
use multiemu_machine::{
    memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable},
    processor::decoder::InstructionDecoder,
};
use std::ops::Range;
use strum::FromRepr;

#[derive(Debug, Default)]
pub struct M6502InstructionDecoder;

impl InstructionDecoder for M6502InstructionDecoder {
    type InstructionSet = M6502InstructionSet;

    fn decode(
        &self,
        cursor: usize,
        address_space: AddressSpaceId,
        memory_translation_table: &MemoryTranslationTable,
    ) -> Option<(Self::InstructionSet, u8)> {
        let mut cursor = cursor as u16;
        let mut instruction_first_byte = 0;
        let _ = memory_translation_table.read(
            cursor as usize,
            address_space,
            std::array::from_mut(&mut instruction_first_byte),
        );
        cursor = cursor.wrapping_add(1);

        let instruction_first_byte = instruction_first_byte.view_bits::<Msb0>();
        let instruction_identifier = InstructionGroup::from_repr(
            instruction_first_byte[INSTRUCTION_IDENTIFIER].load::<u8>(),
        )
        .unwrap();
        let secondary_instruction_identifier =
            instruction_first_byte[SECONDARY_INSTRUCTION_IDENTIFIER].load::<u8>();

        Some(match instruction_identifier {
            InstructionGroup::Group3 => decode_group3_space_instruction(
                cursor,
                address_space,
                memory_translation_table,
                secondary_instruction_identifier,
                instruction_first_byte,
            ),
            InstructionGroup::Group1 => decode_group1_space_instruction(
                cursor,
                address_space,
                memory_translation_table,
                secondary_instruction_identifier,
                instruction_first_byte,
            ),
            InstructionGroup::Group2 => decode_group2_space_instruction(
                cursor,
                address_space,
                memory_translation_table,
                secondary_instruction_identifier,
                instruction_first_byte,
            ),
            InstructionGroup::Undocumented => decode_undocumented_space_instruction(
                cursor,
                address_space,
                memory_translation_table,
                secondary_instruction_identifier,
                instruction_first_byte,
            ),
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
            1 + added_instruction_length,
        ),
        0b001 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::And,
                addressing_mode: Some(addressing_mode),
            },
            1 + added_instruction_length,
        ),
        0b010 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Eor,
                addressing_mode: Some(addressing_mode),
            },
            1 + added_instruction_length,
        ),
        0b011 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Adc,
                addressing_mode: Some(addressing_mode),
            },
            1 + added_instruction_length,
        ),
        0b100 => {
            if matches!(addressing_mode, AddressingMode::Immediate(..)) {
                // STA immediate is NOP
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Sta,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            }
        }
        0b101 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Lda,
                addressing_mode: Some(addressing_mode),
            },
            1 + added_instruction_length,
        ),
        0b110 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Cmp,
                addressing_mode: Some(addressing_mode),
            },
            1 + added_instruction_length,
        ),
        0b111 => (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Sbc,
                addressing_mode: Some(addressing_mode),
            },
            1 + added_instruction_length,
        ),
        _ => {
            unreachable!()
        }
    }
}

#[inline]
pub fn decode_group2_space_instruction(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (M6502InstructionSet, u8) {
    let addressing_mode_byte = instruction_first_byte[ARGUMENT].load::<u8>();

    let (addressing_mode, added_instruction_length) = AddressingMode::from_group2_addressing(
        cursor,
        address_space,
        memory_translation_table,
        addressing_mode_byte,
    );

    // Any case of ZeroPageIndirectYIndexed here is JAM
    if matches!(
        addressing_mode,
        AddressingMode::ZeroPageIndirectYIndexed(..)
    ) // A few cases of XIndexedZeroPageIndirect here are JAM
     || ([0b000, 0b001, 0b010, 0b011].contains(&instruction_identifier)
        && matches!(
            addressing_mode,
            AddressingMode::XIndexedZeroPageIndirect(..)
        ))
    {
        return (
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Jam,
                addressing_mode: None,
            },
            1,
        );
    }

    match instruction_identifier {
        0b000 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    1,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Asl,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            }
        }
        0b001 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    1,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Rol,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            }
        }
        0b010 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    1,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Lsr,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            }
        }
        0b011 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    1,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Ror,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            }
        }
        0b100 => todo!(),
        0b101 => todo!(),
        0b110 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    1,
                )
            } else {
                todo!()
            }
        }
        0b111 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute(..)) {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Nop,
                        addressing_mode: None,
                    },
                    1,
                )
            } else {
                todo!()
            }
        }
        _ => {
            unreachable!()
        }
    }
}

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
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Slo,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
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
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Rla,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
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
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Sre,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
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
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Rra,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            }
        }
        0b100 => match addressing_mode {
            AddressingMode::Immediate(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Xaa,
                    addressing_mode: Some(addressing_mode),
                },
                1 + added_instruction_length,
            ),
            AddressingMode::Absolute(..)
            | AddressingMode::ZeroPage(..)
            | AddressingMode::YIndexedZeroPage(..)
            | AddressingMode::XIndexedZeroPage(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sax,
                    addressing_mode: Some(addressing_mode),
                },
                1 + added_instruction_length,
            ),
            AddressingMode::YIndexedAbsolute(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Shs,
                    addressing_mode: Some(addressing_mode),
                },
                1 + added_instruction_length,
            ),
            AddressingMode::XIndexedAbsolute(address) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sha,
                    addressing_mode: Some(AddressingMode::YIndexedAbsolute(address)),
                },
                1 + added_instruction_length,
            ),
            AddressingMode::ZeroPageIndirectYIndexed(..) => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Sha,
                    addressing_mode: Some(addressing_mode),
                },
                1 + added_instruction_length,
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
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Lax,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
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
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Dcp,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
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
                    1 + added_instruction_length,
                )
            } else {
                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Isc,
                        addressing_mode: Some(addressing_mode),
                    },
                    1 + added_instruction_length,
                )
            }
        }
        _ => {
            unreachable!()
        }
    }
}

#[inline]
fn decode_group3_space_instruction(
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
                1,
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
                    1 + added_instruction_length,
                )
            }
            0b010 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Php,
                    addressing_mode: None,
                },
                1,
            ),
            0b100 => {
                let mut relative = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    bytemuck::bytes_of_mut(&mut relative),
                );

                (
                    M6502InstructionSet {
                        specifier: M6502InstructionSetSpecifier::Bpl,
                        addressing_mode: Some(AddressingMode::Relative(relative)),
                    },
                    2,
                )
            }
            0b110 => (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Clc,
                    addressing_mode: None,
                },
                1,
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
                    3,
                )
            }
            0b001 => todo!(),
            0b010 => todo!(),
            0b011 => todo!(),
            0b100 => todo!(),
            0b101 => todo!(),
            0b110 => todo!(),
            0b111 => todo!(),
            _ => {
                unreachable!()
            }
        },
        0b010 => todo!(),
        0b011 => todo!(),
        0b100 => todo!(),
        0b101 => todo!(),
        0b110 => todo!(),
        0b111 => todo!(),
        _ => {
            unreachable!()
        }
    }
}
