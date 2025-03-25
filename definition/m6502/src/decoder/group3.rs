use super::ARGUMENT;
use crate::instruction::{AddressingMode, Opcode};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};

// This one is hellish to decode

#[inline]
pub fn decode_group3_space_instruction(
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (Opcode, Option<AddressingMode>) {
    let argument = instruction_first_byte[ARGUMENT].load::<u8>();
    let addressing_mode = AddressingMode::from_group1_addressing(argument);

    match instruction_identifier {
        0b000 => match argument {
            0b000 => (Opcode::Brk, None),
            0b001 | 0b011 | 0b101 | 0b111 => (Opcode::Nop, Some(addressing_mode)),
            0b010 => (Opcode::Php, None),
            0b100 => (Opcode::Bpl, Some(AddressingMode::Relative)),
            0b110 => (Opcode::Clc, None),
            _ => {
                unreachable!()
            }
        },
        0b001 => match argument {
            0b000 => (Opcode::Jsr, Some(AddressingMode::Absolute)),
            0b001 | 0b011 => (Opcode::Bit, Some(addressing_mode)),
            0b010 => (Opcode::Plp, None),
            0b100 => (Opcode::Bmi, Some(AddressingMode::Relative)),
            0b101 | 0b111 => (Opcode::Nop, Some(addressing_mode)),
            0b110 => (Opcode::Sec, None),
            _ => {
                unreachable!()
            }
        },
        0b010 => match argument {
            0b000 => (Opcode::Rti, None),
            0b001 | 0b101 | 0b111 => (Opcode::Nop, Some(addressing_mode)),
            0b010 => (Opcode::Pha, None),
            0b011 => (Opcode::Jmp, Some(AddressingMode::Absolute)),
            0b100 => (Opcode::Bvc, Some(AddressingMode::Relative)),
            0b110 => (Opcode::Cli, None),
            _ => {
                unreachable!()
            }
        },
        0b011 => match argument {
            0b000 => (Opcode::Rts, None),
            0b001 | 0b101 | 0b111 => (Opcode::Nop, Some(addressing_mode)),
            0b010 => (Opcode::Pla, None),
            0b011 => (Opcode::Jmp, Some(AddressingMode::AbsoluteIndirect)),
            0b100 => (Opcode::Bvs, Some(AddressingMode::Relative)),
            0b110 => (Opcode::Sei, None),
            _ => {
                unreachable!()
            }
        },
        0b100 => match argument {
            0b000 => (Opcode::Nop, Some(AddressingMode::Immediate)),
            0b001 | 0b011 | 0b101 => (Opcode::Sty, Some(addressing_mode)),
            0b010 => (Opcode::Dey, None),
            0b100 => (Opcode::Bcc, Some(AddressingMode::Relative)),
            0b110 => (Opcode::Tya, None),
            0b111 => (Opcode::Shy, Some(addressing_mode)),
            _ => {
                unreachable!()
            }
        },
        0b101 => match argument {
            0b000 => (Opcode::Ldy, Some(AddressingMode::Immediate)),
            0b001 | 0b011 | 0b101 | 0b111 => (Opcode::Ldy, Some(addressing_mode)),
            0b010 => (Opcode::Tay, None),
            0b100 => (Opcode::Bcs, Some(AddressingMode::Relative)),
            0b110 => (Opcode::Clv, None),
            _ => {
                unreachable!()
            }
        },
        0b110 => match argument {
            0b000 => (Opcode::Cpy, Some(AddressingMode::Immediate)),
            0b001 | 0b011 => (Opcode::Cpy, Some(addressing_mode)),
            0b010 => (Opcode::Iny, None),
            0b100 => (Opcode::Bne, Some(AddressingMode::Relative)),
            0b101 | 0b111 => (Opcode::Nop, Some(addressing_mode)),
            0b110 => (Opcode::Cld, None),
            _ => {
                unreachable!()
            }
        },
        0b111 => match argument {
            0b000 => (Opcode::Cpx, Some(AddressingMode::Immediate)),
            0b001 | 0b011 => (Opcode::Cpx, Some(addressing_mode)),
            0b010 => (Opcode::Inx, None),
            0b100 => (Opcode::Beq, Some(AddressingMode::Relative)),
            0b101 | 0b111 => (Opcode::Nop, Some(addressing_mode)),
            0b110 => (Opcode::Sed, None),
            _ => {
                unreachable!()
            }
        },
        _ => {
            unreachable!()
        }
    }
}
