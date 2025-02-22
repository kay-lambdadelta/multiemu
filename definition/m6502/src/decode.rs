use super::instruction::{AddressingMode, M6502InstructionSet, M6502InstructionSetSpecifier};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
    view::BitView,
};
use multiemu_machine::memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable};
use std::ops::Range;
use strum::FromRepr;

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

pub fn decode_instruction(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
) -> Result<(M6502InstructionSet, u8), Box<dyn std::error::Error>> {
    let mut instruction_first_byte = 0;
    memory_translation_table.read(
        cursor as usize,
        address_space,
        std::array::from_mut(&mut instruction_first_byte),
    )?;

    let instruction_first_byte = instruction_first_byte.view_bits::<Msb0>();
    let instruction_identifier =
        InstructionGroup::from_repr(instruction_first_byte[INSTRUCTION_IDENTIFIER].load::<u8>())
            .unwrap();
    let secondary_instruction_identifier =
        instruction_first_byte[SECONDARY_INSTRUCTION_IDENTIFIER].load::<u8>();

    match instruction_identifier {
        InstructionGroup::Group3 => decode_group3_space_instruction(
            cursor,
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
            memory_translation_table,
            secondary_instruction_identifier,
            instruction_first_byte,
        ),
        InstructionGroup::Undocumented => decode_undocumented_space_instruction(
            cursor,
            memory_translation_table,
            secondary_instruction_identifier,
            instruction_first_byte,
        ),
    }
}

pub fn decode_group1_space_instruction(
    cursor: u16,
    address_space: AddressSpaceId,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> Result<(M6502InstructionSet, u8), Box<dyn std::error::Error>> {
    let addressing_mode = instruction_first_byte[ARGUMENT].load::<u8>();

    match instruction_identifier {
        0b000 => {
            let (addressing_mode, added_instruction_length) =
                AddressingMode::from_group1_addressing(
                    cursor,
                    address_space,
                    memory_translation_table,
                    addressing_mode,
                );

            Ok((
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(addressing_mode),
                },
                1 + added_instruction_length,
            ))
        }
        0b001 => {
            todo!()
        }
        0b010 => {
            todo!()
        }
        0b011 => {
            todo!()
        }
        0b100 => {
            todo!()
        }
        0b101 => {
            todo!()
        }
        0b110 => {
            todo!()
        }
        0b111 => {
            todo!()
        }
        _ => {
            unreachable!()
        }
    }
}

#[inline]
pub fn decode_group2_space_instruction(
    cursor: u16,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> Result<(M6502InstructionSet, u8), Box<dyn std::error::Error>> {
    todo!()
}

#[inline]
pub fn decode_undocumented_space_instruction(
    cursor: u16,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> Result<(M6502InstructionSet, u8), Box<dyn std::error::Error>> {
    match instruction_identifier {
        0b000 => {
            todo!()
        }
        0b001 => {
            todo!()
        }
        0b010 => {
            todo!()
        }
        0b011 => {
            todo!()
        }
        0b100 => {
            todo!()
        }
        0b101 => {
            todo!()
        }
        0b110 => {
            todo!()
        }
        0b111 => {
            todo!()
        }
        _ => {
            unreachable!()
        }
    }
}

fn decode_group3_space_instruction(
    cursor: u16,
    memory_translation_table: &MemoryTranslationTable,
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> Result<(M6502InstructionSet, u8), Box<dyn std::error::Error>> {
    let addressing_mode = instruction_first_byte[ARGUMENT].load::<u8>();

    match instruction_identifier {
        0b000 => Ok((
            M6502InstructionSet {
                specifier: M6502InstructionSetSpecifier::Brk,
                addressing_mode: None,
            },
            1,
        )),
        0b001 => {
            todo!()
        }
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
