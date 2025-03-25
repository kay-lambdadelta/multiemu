use super::ARGUMENT;
use crate::instruction::{AddressingMode, Opcode};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};

#[inline]
pub fn decode_undocumented_space_instruction(
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (Opcode, Option<AddressingMode>) {
    let argument = instruction_first_byte[ARGUMENT].load::<u8>();

    let addressing_mode = AddressingMode::from_group1_addressing(argument);

    match instruction_identifier {
        0b000 => {
            if matches!(addressing_mode, AddressingMode::Immediate) {
                (Opcode::Anc, Some(addressing_mode))
            } else {
                (Opcode::Slo, Some(addressing_mode))
            }
        }
        0b001 => {
            if matches!(addressing_mode, AddressingMode::Immediate) {
                (Opcode::Anc, Some(addressing_mode))
            } else {
                (Opcode::Rla, Some(addressing_mode))
            }
        }
        0b010 => {
            if matches!(addressing_mode, AddressingMode::Immediate) {
                (Opcode::Anc, Some(addressing_mode))
            } else {
                (Opcode::Sre, Some(addressing_mode))
            }
        }
        0b011 => {
            if matches!(addressing_mode, AddressingMode::Immediate) {
                (Opcode::Anc, Some(addressing_mode))
            } else {
                (Opcode::Rra, Some(addressing_mode))
            }
        }
        0b100 => match addressing_mode {
            AddressingMode::XIndexedZeroPageIndirect => (Opcode::Sax, Some(addressing_mode)),
            AddressingMode::Immediate => (Opcode::Xaa, Some(addressing_mode)),
            AddressingMode::Absolute | AddressingMode::ZeroPage => {
                (Opcode::Sax, Some(addressing_mode))
            }
            AddressingMode::XIndexedZeroPage => {
                (Opcode::Sax, Some(AddressingMode::YIndexedZeroPage))
            }
            AddressingMode::YIndexedAbsolute => (Opcode::Shs, Some(addressing_mode)),
            AddressingMode::XIndexedAbsolute => {
                (Opcode::Sha, Some(AddressingMode::YIndexedAbsolute))
            }
            AddressingMode::ZeroPageIndirectYIndexed => (Opcode::Sha, Some(addressing_mode)),
            _ => unreachable!(),
        },
        0b101 => {
            if matches!(addressing_mode, AddressingMode::YIndexedAbsolute) {
                (Opcode::Las, Some(addressing_mode))
            } else {
                (Opcode::Lax, Some(addressing_mode))
            }
        }
        0b110 => {
            if matches!(addressing_mode, AddressingMode::Immediate) {
                (Opcode::Sbx, Some(addressing_mode))
            } else {
                (Opcode::Dcp, Some(addressing_mode))
            }
        }
        0b111 => {
            if matches!(addressing_mode, AddressingMode::Immediate) {
                (Opcode::Sbc, Some(addressing_mode))
            } else {
                (Opcode::Isc, Some(addressing_mode))
            }
        }
        _ => {
            unreachable!()
        }
    }
}
