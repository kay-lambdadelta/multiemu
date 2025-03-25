use super::ARGUMENT;
use crate::instruction::{AddressingMode, Opcode};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};

#[inline]
pub fn decode_group1_space_instruction(
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
) -> (Opcode, Option<AddressingMode>) {
    let addressing_mode = instruction_first_byte[ARGUMENT].load::<u8>();

    let addressing_mode = AddressingMode::from_group1_addressing(addressing_mode);

    match instruction_identifier {
        0b000 => (Opcode::Ora, Some(addressing_mode)),
        0b001 => (Opcode::And, Some(addressing_mode)),
        0b010 => (Opcode::Eor, Some(addressing_mode)),
        0b011 => (Opcode::Adc, Some(addressing_mode)),
        0b100 => {
            if matches!(addressing_mode, AddressingMode::Immediate) {
                // STA immediate is NOP
                (Opcode::Nop, Some(addressing_mode))
            } else {
                (Opcode::Sta, Some(addressing_mode))
            }
        }
        0b101 => (Opcode::Lda, Some(addressing_mode)),
        0b110 => (Opcode::Cmp, Some(addressing_mode)),
        0b111 => (Opcode::Sbc, Some(addressing_mode)),
        _ => {
            unreachable!()
        }
    }
}
