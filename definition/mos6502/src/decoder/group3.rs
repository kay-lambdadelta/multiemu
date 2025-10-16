use super::ARGUMENT;
use crate::{
    Mos6502Kind,
    instruction::{AddressingMode, Mos6502AddressingMode, Mos6502Opcode, Opcode},
};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};

// This one is hellish to decode

#[inline]
pub fn decode_group3_space_instruction(
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
    _kind: Mos6502Kind,
) -> (Opcode, Option<AddressingMode>) {
    let argument = instruction_first_byte[ARGUMENT].load::<u8>();
    let addressing_mode = AddressingMode::from_group1_addressing(argument);

    match instruction_identifier {
        0b000 => match argument {
            0b000 => (Opcode::Mos6502(Mos6502Opcode::Brk), None),
            0b001 | 0b011 | 0b101 | 0b111 => {
                (Opcode::Mos6502(Mos6502Opcode::Nop), Some(addressing_mode))
            }
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Php), None),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Bpl),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Clc), None),
            _ => {
                unreachable!()
            }
        },
        0b001 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Jsr),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Absolute)),
            ),
            0b001 | 0b011 => (Opcode::Mos6502(Mos6502Opcode::Bit), Some(addressing_mode)),
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Plp), None),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Bmi),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b101 | 0b111 => (Opcode::Mos6502(Mos6502Opcode::Nop), Some(addressing_mode)),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Sec), None),
            _ => {
                unreachable!()
            }
        },
        0b010 => match argument {
            0b000 => (Opcode::Mos6502(Mos6502Opcode::Rti), None),
            0b001 | 0b101 | 0b111 => (Opcode::Mos6502(Mos6502Opcode::Nop), Some(addressing_mode)),
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Pha), None),
            0b011 => (
                Opcode::Mos6502(Mos6502Opcode::Jmp),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Absolute)),
            ),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Bvc),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Cli), None),
            _ => {
                unreachable!()
            }
        },
        0b011 => match argument {
            0b000 => (Opcode::Mos6502(Mos6502Opcode::Rts), None),
            0b001 | 0b101 | 0b111 => (Opcode::Mos6502(Mos6502Opcode::Nop), Some(addressing_mode)),
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Pla), None),
            0b011 => (
                Opcode::Mos6502(Mos6502Opcode::Jmp),
                Some(AddressingMode::Mos6502(
                    Mos6502AddressingMode::AbsoluteIndirect,
                )),
            ),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Bvs),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Sei), None),
            _ => {
                unreachable!()
            }
        },
        0b100 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Nop),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
            ),
            0b001 | 0b011 | 0b101 => (Opcode::Mos6502(Mos6502Opcode::Sty), Some(addressing_mode)),
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Dey), None),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Bcc),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Tya), None),
            0b111 => (Opcode::Mos6502(Mos6502Opcode::Shy), Some(addressing_mode)),
            _ => {
                unreachable!()
            }
        },
        0b101 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Ldy),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
            ),
            0b001 | 0b011 | 0b101 | 0b111 => {
                (Opcode::Mos6502(Mos6502Opcode::Ldy), Some(addressing_mode))
            }
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Tay), None),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Bcs),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Clv), None),
            _ => {
                unreachable!()
            }
        },
        0b110 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Cpy),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
            ),
            0b001 | 0b011 => (Opcode::Mos6502(Mos6502Opcode::Cpy), Some(addressing_mode)),
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Iny), None),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Bne),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b101 | 0b111 => (Opcode::Mos6502(Mos6502Opcode::Nop), Some(addressing_mode)),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Cld), None),
            _ => {
                unreachable!()
            }
        },
        0b111 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Cpx),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
            ),
            0b001 | 0b011 => (Opcode::Mos6502(Mos6502Opcode::Cpx), Some(addressing_mode)),
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Inx), None),
            0b100 => (
                Opcode::Mos6502(Mos6502Opcode::Beq),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative)),
            ),
            0b101 | 0b111 => (Opcode::Mos6502(Mos6502Opcode::Nop), Some(addressing_mode)),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Sed), None),
            _ => {
                unreachable!()
            }
        },
        _ => {
            unreachable!()
        }
    }
}
