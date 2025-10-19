use crate::{
    Mos6502Kind,
    instruction::{AddressingMode, Mos6502AddressingMode, Mos6502Opcode, Opcode},
};

#[inline]
pub fn decode_group2_space_instruction(
    instruction_identifier: u8,
    argument: u8,
    kind: Mos6502Kind,
) -> (Opcode, Option<AddressingMode>) {
    let addressing_mode = AddressingMode::from_group2_addressing(argument, kind);

    match instruction_identifier {
        0b000 => match argument {
            0b000 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Nop),
                        Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Ora),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Nop), None),
            _ => (
                Opcode::Mos6502(Mos6502Opcode::Asl),
                Some(addressing_mode.unwrap()),
            ),
        },
        0b001 => match argument {
            0b000 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Nop),
                        Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::And),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Nop), None),
            _ => (
                Opcode::Mos6502(Mos6502Opcode::Rol),
                Some(addressing_mode.unwrap()),
            ),
        },
        0b010 => match argument {
            0b000 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Nop),
                        Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Eor),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Nop), None),
            _ => (
                Opcode::Mos6502(Mos6502Opcode::Lsr),
                Some(addressing_mode.unwrap()),
            ),
        },
        0b011 => match argument {
            0b000 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Nop),
                        Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Adc),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Nop), None),
            _ => (
                Opcode::Mos6502(Mos6502Opcode::Ror),
                Some(addressing_mode.unwrap()),
            ),
        },
        0b100 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Nop),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
            ),
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Sta),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b001 | 0b011 | 0b101 => {
                // STX uses YIndexedZeroPage instead of XIndexedZeroPage
                let fixed_mode = if addressing_mode
                    == Some(AddressingMode::Mos6502(
                        Mos6502AddressingMode::XIndexedZeroPage,
                    )) {
                    Some(AddressingMode::Mos6502(
                        Mos6502AddressingMode::YIndexedZeroPage,
                    ))
                } else {
                    addressing_mode
                };

                (Opcode::Mos6502(Mos6502Opcode::Stx), fixed_mode)
            }
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Txa), None),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Txs), None),
            0b111 => (
                Opcode::Mos6502(Mos6502Opcode::Shx),
                Some(addressing_mode.unwrap()),
            ),
            _ => unreachable!(),
        },
        0b101 => match argument {
            0b000 | 0b001 | 0b011 | 0b101 | 0b111 => {
                // Ldx uses YIndexedZeroPage instead of XIndexedZeroPage
                let fixed_mode = match addressing_mode {
                    Some(AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage)) => Some(
                        AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedZeroPage),
                    ),
                    Some(AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute)) => Some(
                        AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute),
                    ),
                    _ => addressing_mode,
                };

                (Opcode::Mos6502(Mos6502Opcode::Ldx), fixed_mode)
            }
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Lda),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Tax), None),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Tsx), None),
            _ => unreachable!(),
        },
        0b110 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Nop),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
            ),
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Cmp),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b001 | 0b011 | 0b101 | 0b111 => (
                Opcode::Mos6502(Mos6502Opcode::Dec),
                Some(addressing_mode.unwrap()),
            ),
            0b010 => (Opcode::Mos6502(Mos6502Opcode::Dex), None),
            0b110 => (Opcode::Mos6502(Mos6502Opcode::Nop), None),
            _ => unreachable!(),
        },
        0b111 => match argument {
            0b000 => (
                Opcode::Mos6502(Mos6502Opcode::Nop),
                Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)),
            ),
            0b100 => {
                if kind == Mos6502Kind::Wdc65C02 {
                    (
                        Opcode::Mos6502(Mos6502Opcode::Sbc),
                        Some(addressing_mode.unwrap()),
                    )
                } else {
                    (Opcode::Mos6502(Mos6502Opcode::Jam), None)
                }
            }
            0b001 | 0b011 | 0b101 | 0b111 => (
                Opcode::Mos6502(Mos6502Opcode::Inc),
                Some(addressing_mode.unwrap()),
            ),
            0b010 | 0b110 => (Opcode::Mos6502(Mos6502Opcode::Nop), None),
            _ => unreachable!(),
        },
        _ => {
            unreachable!()
        }
    }
}
