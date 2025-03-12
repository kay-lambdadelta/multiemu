use super::instruction::{AddressingMode, M6502InstructionSet};
use bitvec::{field::BitField, prelude::Msb0, view::BitView};
use group1::decode_group1_space_instruction;
use group2::decode_group2_space_instruction;
use group3::decode_group3_space_instruction;
use multiemu_machine::{
    memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable},
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

        let result = Some(match instruction_identifier {
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
        });

        result.map(|(instruction_set, added_instruction_length)| {
            if instruction_set.addressing_mode.is_none() {
                debug_assert!(added_instruction_length == 0);
            }

            (instruction_set, added_instruction_length + 1)
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
    pub fn from_group1_addressing(
        cursor: u16,
        address_space: AddressSpaceId,
        memory_translation_table: &MemoryTranslationTable,
        addressing_mode: u8,
    ) -> (Self, u8) {
        match addressing_mode {
            0b000 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::XIndexedZeroPageIndirect(indirect), 1)
            }
            0b001 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPage(indirect), 1)
            }
            0b010 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut immediate),
                );

                (AddressingMode::Immediate(immediate), 1)
            }
            0b011 => {
                let mut indirect = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut indirect);

                (AddressingMode::Absolute(u16::from_le_bytes(indirect)), 2)
            }
            0b100 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPageIndirectYIndexed(indirect), 1)
            }
            0b101 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::XIndexedZeroPage(indirect), 1)
            }
            0b110 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::YIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            0b111 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::XIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            _ => {
                unreachable!()
            }
        }
    }

    #[inline]
    pub fn from_group2_addressing(
        cursor: u16,
        address_space: AddressSpaceId,
        memory_translation_table: &MemoryTranslationTable,
        addressing_mode: u8,
    ) -> (Self, u8) {
        match addressing_mode {
            0b000 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut immediate),
                );

                (AddressingMode::Immediate(immediate), 1)
            }
            0b001 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPage(indirect), 1)
            }
            0b010 => (AddressingMode::Accumulator, 0),
            0b011 => {
                let mut indirect = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut indirect);

                (AddressingMode::Absolute(u16::from_le_bytes(indirect)), 2)
            }
            0b100 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPageIndirectYIndexed(indirect), 1)
            }
            0b101 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::XIndexedZeroPage(indirect), 1)
            }
            0b110 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::YIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            0b111 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::XIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            _ => {
                unreachable!()
            }
        }
    }
}
