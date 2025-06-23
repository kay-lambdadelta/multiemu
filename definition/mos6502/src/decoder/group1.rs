use super::ARGUMENT;
use crate::{
    Mos6502Kind,
    instruction::{AddressingMode, Mos6502AddressingMode, Mos6502Opcode, Opcode},
};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};

#[inline]
pub fn decode_group1_space_instruction(
    instruction_identifier: u8,
    instruction_first_byte: &BitSlice<u8, Msb0>,
    kind: Mos6502Kind,
) -> (Opcode, Option<AddressingMode>) {
    let addressing_mode = instruction_first_byte[ARGUMENT].load::<u8>();

    let addressing_mode = AddressingMode::from_group1_addressing(addressing_mode);

    match instruction_identifier {
        0b000 => (Opcode::Mos6502(Mos6502Opcode::Ora), Some(addressing_mode)),
        0b001 => (Opcode::Mos6502(Mos6502Opcode::And), Some(addressing_mode)),
        0b010 => (Opcode::Mos6502(Mos6502Opcode::Eor), Some(addressing_mode)),
        0b011 => (Opcode::Mos6502(Mos6502Opcode::Adc), Some(addressing_mode)),
        0b100 => {
            if addressing_mode == AddressingMode::Mos6502(Mos6502AddressingMode::Immediate) {
                if kind == Mos6502Kind::Wdc65C02 {
                    (Opcode::Mos6502(Mos6502Opcode::Bit), Some(addressing_mode))
                } else {
                    // This is STA immediate which is a NOP
                    (Opcode::Mos6502(Mos6502Opcode::Nop), Some(addressing_mode))
                }
            } else {
                (Opcode::Mos6502(Mos6502Opcode::Sta), Some(addressing_mode))
            }
        }
        0b101 => (Opcode::Mos6502(Mos6502Opcode::Lda), Some(addressing_mode)),
        0b110 => (Opcode::Mos6502(Mos6502Opcode::Cmp), Some(addressing_mode)),
        0b111 => (Opcode::Mos6502(Mos6502Opcode::Sbc), Some(addressing_mode)),
        _ => {
            unreachable!()
        }
    }
}
