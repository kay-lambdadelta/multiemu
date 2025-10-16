use super::instruction::{AddressingMode, Mos6502InstructionSet};
use crate::{
    Mos6502Kind,
    instruction::{Mos6502AddressingMode, Wdc65C02AddressingMode},
};
use bitvec::{field::BitField, prelude::Msb0, view::BitView};
use group1::decode_group1_space_instruction;
use group2::decode_group2_space_instruction;
use group3::decode_group3_space_instruction;
use multiemu_base::{
    memory::{Address, AddressSpaceId, MemoryAccessTable},
    processor::InstructionDecoder,
};
use std::ops::Range;
use strum::FromRepr;
use undocumented::decode_undocumented_space_instruction;

mod group1;
mod group2;
mod group3;
mod undocumented;
#[cfg(test)]
mod test;

// https://www.pagetable.com/c64ref/6502/

#[derive(Debug)]
pub struct Mos6502InstructionDecoder {
    kind: Mos6502Kind,
}

impl Mos6502InstructionDecoder {
    pub fn new(kind: Mos6502Kind) -> Self {
        Self { kind }
    }
}

impl InstructionDecoder for Mos6502InstructionDecoder {
    type InstructionSet = Mos6502InstructionSet;

    #[inline]
    fn decode(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        memory_access_table: &MemoryAccessTable,
    ) -> Option<(Self::InstructionSet, u8)> {
        let byte: u8 = memory_access_table
            .read_le_value(address, address_space)
            .unwrap_or_default();

        let byte = byte.view_bits::<Msb0>();
        let instruction_identifier =
            InstructionGroup::from_repr(byte[INSTRUCTION_IDENTIFIER].load::<u8>()).unwrap();
        let secondary_instruction_identifier = byte[SECONDARY_INSTRUCTION_IDENTIFIER].load::<u8>();

        let result = Some(match instruction_identifier {
            InstructionGroup::Group3 => {
                decode_group3_space_instruction(secondary_instruction_identifier, byte, self.kind)
            }
            InstructionGroup::Group1 => {
                decode_group1_space_instruction(secondary_instruction_identifier, byte, self.kind)
            }
            InstructionGroup::Group2 => {
                decode_group2_space_instruction(secondary_instruction_identifier, byte, self.kind)
            }
            InstructionGroup::Undocumented => decode_undocumented_space_instruction(
                secondary_instruction_identifier,
                byte,
                self.kind,
            ),
        });

        result.map(|(opcode, addressing_mode)| {
            (
                Mos6502InstructionSet {
                    opcode,
                    addressing_mode,
                },
                // This is not how long the instruction is, but how many bytes were read to gather this information
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

// The names of these addressing mode translation functions don't really and truly mean much

impl AddressingMode {
    #[inline]
    pub fn from_group1_addressing(addressing_mode: u8) -> Self {
        match addressing_mode {
            0b000 => AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPageIndirect),
            0b001 => AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPage),
            0b010 => AddressingMode::Mos6502(Mos6502AddressingMode::Immediate),
            0b011 => AddressingMode::Mos6502(Mos6502AddressingMode::Absolute),
            0b100 => AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPageIndirectYIndexed),
            0b101 => AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage),
            0b110 => AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute),
            0b111 => AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute),
            _ => {
                unreachable!()
            }
        }
    }

    #[inline]
    pub fn from_group2_addressing(addressing_mode: u8, kind: Mos6502Kind) -> Option<Self> {
        Some(match addressing_mode {
            0b000 => AddressingMode::Mos6502(Mos6502AddressingMode::Immediate),
            0b001 => AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPage),
            0b010 => AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator),
            0b011 => AddressingMode::Mos6502(Mos6502AddressingMode::Absolute),
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    AddressingMode::Wdc65C02(Wdc65C02AddressingMode::ZeroPageIndirect)
                } else {
                    // Above parsing code should turn this into a JAM
                    return None;
                }
            }
            0b101 => AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage),
            0b110 => AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute),
            0b111 => AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute),
            _ => {
                unreachable!()
            }
        })
    }
}
