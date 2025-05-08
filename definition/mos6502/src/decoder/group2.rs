use super::ARGUMENT;
use crate::instruction::{AddressingMode, Opcode};
use bitvec::{
    field::BitField,
    prelude::{BitSlice, Msb0},
};

#[inline]
pub fn decode_group2_space_instruction(
    instruction_identifier: u8,
    instruction_byte: &BitSlice<u8, Msb0>,
) -> (Opcode, Option<AddressingMode>) {
    let argument = instruction_byte[ARGUMENT].load::<u8>();
    let addressing_mode = AddressingMode::from_group2_addressing(argument);

    // Any case of ZeroPageIndirectYIndexed here is JAM
    if (
        argument == 0b100
    ) // A few cases of XIndexedZeroPageIndirect here are JAM
     || ([0b000, 0b001, 0b010, 0b011].contains(&instruction_identifier)
        && (
            argument ==
            0b000
        ))
    {
        return (Opcode::Jam, None);
    }

    match instruction_identifier {
        0b000 => {
            if argument == 0b110 {
                (Opcode::Nop, None)
            } else {
                (Opcode::Asl, Some(addressing_mode))
            }
        }
        0b001 => {
            if argument == 0b110 {
                (Opcode::Nop, None)
            } else {
                (Opcode::Rol, Some(addressing_mode))
            }
        }
        0b010 => {
            if argument == 0b110 {
                (Opcode::Nop, None)
            } else {
                (Opcode::Lsr, Some(addressing_mode))
            }
        }
        0b011 => {
            if argument == 0b110 {
                (Opcode::Nop, None)
            } else {
                (Opcode::Ror, Some(addressing_mode))
            }
        }
        0b100 => match argument {
            0b000 => (Opcode::Nop, Some(AddressingMode::Immediate)),
            0b001 | 0b011 | 0b101 => (Opcode::Stx, Some(addressing_mode)),
            0b010 => (Opcode::Txa, None),
            0b110 => (Opcode::Txs, None),
            0b111 => (Opcode::Shx, Some(addressing_mode)),
            _ => unreachable!(),
        },
        0b101 => match argument {
            0b000 | 0b001 | 0b011 | 0b101 | 0b111 => (Opcode::Ldx, Some(addressing_mode)),
            0b010 => (Opcode::Tax, None),
            0b110 => (Opcode::Tsx, None),
            _ => unreachable!(),
        },
        0b110 => match argument {
            0b000 => (Opcode::Nop, Some(AddressingMode::Immediate)),
            0b001 | 0b011 | 0b101 | 0b111 => (Opcode::Dec, Some(addressing_mode)),
            0b010 => (Opcode::Dex, None),
            0b110 => (Opcode::Nop, None),
            _ => unreachable!(),
        },
        0b111 => match argument {
            0b000 => (Opcode::Nop, Some(AddressingMode::Immediate)),
            0b001 | 0b011 | 0b101 | 0b111 => (Opcode::Inc, Some(addressing_mode)),
            0b010 | 0b110 => (Opcode::Nop, None),
            _ => unreachable!(),
        },
        _ => {
            unreachable!()
        }
    }
}
