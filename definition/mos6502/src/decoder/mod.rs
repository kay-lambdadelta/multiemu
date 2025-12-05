pub use group1::decode_group1_space_instruction;
pub use group2::decode_group2_space_instruction;
pub use group3::decode_group3_space_instruction;
use strum::FromRepr;
pub use undocumented::decode_undocumented_space_instruction;

use super::instruction::AddressingMode;
use crate::{
    Mos6502Kind,
    instruction::{Mos6502AddressingMode, Wdc65C02AddressingMode},
};

mod group1;
mod group2;
mod group3;
mod undocumented;

// https://www.pagetable.com/c64ref/6502/

#[derive(FromRepr)]
#[repr(u8)]
pub enum InstructionGroup {
    Group3 = 0b00,
    Group1 = 0b01,
    Group2 = 0b10,
    Undocumented = 0b11,
}

// The names of these addressing mode translation functions don't really and
// truly mean much

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
