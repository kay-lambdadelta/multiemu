use bitvec::{field::BitField, prelude::Msb0, view::BitView};
use multiemu_runtime::{memory::AddressSpace, processor::InstructionDecoder};
use nalgebra::Point2;

use super::instruction::{
    Chip8InstructionSet, InstructionSetChip8, InstructionSetSuperChip8, Register, ScrollDirection,
};

#[derive(Debug, Default)]
pub struct Chip8InstructionDecoder;

impl InstructionDecoder for Chip8InstructionDecoder {
    type InstructionSet = Chip8InstructionSet;

    fn decode(
        &self,
        cursor: usize,
        address_space: &AddressSpace,
    ) -> Option<(Chip8InstructionSet, u8)> {
        let mut instruction = [0; 2];
        address_space.read(cursor, false, &mut instruction).unwrap();

        decode_instruction(instruction).map(|i| (i, 2))
    }
}

fn decode_instruction(instruction: [u8; 2]) -> Option<Chip8InstructionSet> {
    let instruction_view = instruction.view_bits::<Msb0>();

    Some(match instruction_view[0..4].load::<u8>() {
        0x0 => {
            let syscall = [
                instruction_view[4..8].load::<u8>(),
                instruction_view[8..12].load::<u8>(),
                instruction_view[12..16].load::<u8>(),
            ];

            match syscall {
                [0x0, 0xe, 0x0] => Chip8InstructionSet::Chip8(InstructionSetChip8::Clr),
                [0x0, 0xe, 0xe] => Chip8InstructionSet::Chip8(InstructionSetChip8::Rtrn),
                [0x0, 0xf, 0xe] => Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Lores),
                [0x0, 0xf, 0xf] => Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Hires),
                [0x0, 0xc, _] => {
                    Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Scroll {
                        direction: ScrollDirection::Down { amount: syscall[2] },
                    })
                }
                [0x0, 0xf, 0xb] => {
                    Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Scroll {
                        direction: ScrollDirection::Right,
                    })
                }
                [0x0, 0xf, 0xc] => {
                    Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Scroll {
                        direction: ScrollDirection::Right,
                    })
                }
                _ => return None,
            }
        }
        0x1 => {
            let address = instruction_view[4..16].load_be::<u16>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Jump { address })
        }
        0x2 => {
            let address = instruction_view[4..16].load_be::<u16>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Call { address })
        }
        0x3 => {
            let register = instruction_view[4..8].load::<u8>();
            let immediate = instruction_view[8..16].load::<u8>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Ske {
                register: Register::from_repr(register).unwrap(),
                immediate,
            })
        }
        0x4 => {
            let register = instruction_view[4..8].load::<u8>();
            let immediate = instruction_view[8..16].load::<u8>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Skne {
                register: Register::from_repr(register).unwrap(),
                immediate,
            })
        }
        0x5 => {
            let param_1 = instruction_view[4..8].load::<u8>();
            let param_2 = instruction_view[8..12].load::<u8>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Skre {
                param_1: Register::from_repr(param_1).unwrap(),
                param_2: Register::from_repr(param_2).unwrap(),
            })
        }
        0x6 => {
            let register = instruction_view[4..8].load::<u8>();
            let immediate = instruction_view[8..16].load::<u8>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Load {
                register: Register::from_repr(register).unwrap(),
                immediate,
            })
        }
        0x7 => {
            let register = instruction_view[4..8].load::<u8>();
            let immediate = instruction_view[8..16].load::<u8>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Add {
                register: Register::from_repr(register).unwrap(),
                immediate,
            })
        }
        0x8 => {
            let param_1 = instruction_view[4..8].load::<u8>();
            let param_2 = instruction_view[8..12].load::<u8>();

            let specifier = instruction_view[12..16].load::<u8>();

            match specifier {
                0x0 => Chip8InstructionSet::Chip8(InstructionSetChip8::Move {
                    param_1: Register::from_repr(param_1).unwrap(),
                    param_2: Register::from_repr(param_2).unwrap(),
                }),
                0x1 => Chip8InstructionSet::Chip8(InstructionSetChip8::Or {
                    destination: Register::from_repr(param_1).unwrap(),
                    source: Register::from_repr(param_2).unwrap(),
                }),
                0x2 => Chip8InstructionSet::Chip8(InstructionSetChip8::And {
                    destination: Register::from_repr(param_1).unwrap(),
                    source: Register::from_repr(param_2).unwrap(),
                }),
                0x3 => Chip8InstructionSet::Chip8(InstructionSetChip8::Xor {
                    destination: Register::from_repr(param_1).unwrap(),
                    source: Register::from_repr(param_2).unwrap(),
                }),
                0x4 => Chip8InstructionSet::Chip8(InstructionSetChip8::Addr {
                    destination: Register::from_repr(param_1).unwrap(),
                    source: Register::from_repr(param_2).unwrap(),
                }),
                0x5 => Chip8InstructionSet::Chip8(InstructionSetChip8::Sub {
                    destination: Register::from_repr(param_1).unwrap(),
                    source: Register::from_repr(param_2).unwrap(),
                }),
                0x6 => Chip8InstructionSet::Chip8(InstructionSetChip8::Shr {
                    register: Register::from_repr(param_1).unwrap(),
                    value: Register::from_repr(param_2).unwrap(),
                }),
                0x7 => Chip8InstructionSet::Chip8(InstructionSetChip8::Subn {
                    destination: Register::from_repr(param_1).unwrap(),
                    source: Register::from_repr(param_2).unwrap(),
                }),
                0xe => Chip8InstructionSet::Chip8(InstructionSetChip8::Shl {
                    register: Register::from_repr(param_1).unwrap(),
                    value: Register::from_repr(param_2).unwrap(),
                }),
                _ => {
                    return None;
                }
            }
        }
        0x9 => {
            let param_1 = instruction_view[4..8].load::<u8>();
            let param_2 = instruction_view[8..12].load::<u8>();

            match instruction_view[12..16].load::<u8>() {
                0x0 => Chip8InstructionSet::Chip8(InstructionSetChip8::Skrne {
                    param_1: Register::from_repr(param_1).unwrap(),
                    param_2: Register::from_repr(param_2).unwrap(),
                }),
                _ => {
                    return None;
                }
            }
        }
        0xa => {
            let value = instruction_view[4..16].load_be::<u16>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadi { value })
        }
        0xb => {
            let address = instruction_view[4..16].load_be::<u16>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Jumpi { address })
        }
        0xc => {
            let register = instruction_view[4..8].load::<u8>();
            let immediate = instruction_view[8..16].load::<u8>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Rand {
                register: Register::from_repr(register).unwrap(),
                immediate,
            })
        }
        0xd => {
            let x_register = instruction_view[4..8].load::<u8>();
            let y_register = instruction_view[8..12].load::<u8>();
            let height = instruction_view[12..16].load::<u8>();

            Chip8InstructionSet::Chip8(InstructionSetChip8::Draw {
                coordinates: Point2::new(
                    Register::from_repr(x_register).unwrap(),
                    Register::from_repr(y_register).unwrap(),
                ),
                height,
            })
        }
        0xe => {
            let register = instruction_view[4..8].load::<u8>();

            match instruction_view[8..16].load::<u8>() {
                0x9e => Chip8InstructionSet::Chip8(InstructionSetChip8::Skpr {
                    key: Register::from_repr(register).unwrap(),
                }),
                0xa1 => Chip8InstructionSet::Chip8(InstructionSetChip8::Skup {
                    key: Register::from_repr(register).unwrap(),
                }),
                _ => {
                    return None;
                }
            }
        }
        0xf => {
            let register = instruction_view[4..8].load::<u8>();

            match instruction_view[8..16].load::<u8>() {
                0x07 => Chip8InstructionSet::Chip8(InstructionSetChip8::Moved {
                    register: Register::from_repr(register).unwrap(),
                }),
                0x0a => Chip8InstructionSet::Chip8(InstructionSetChip8::Keyd {
                    key: Register::from_repr(register).unwrap(),
                }),
                0x15 => Chip8InstructionSet::Chip8(InstructionSetChip8::Loadd {
                    register: Register::from_repr(register).unwrap(),
                }),
                0x18 => Chip8InstructionSet::Chip8(InstructionSetChip8::Loads {
                    register: Register::from_repr(register).unwrap(),
                }),
                0x1e => Chip8InstructionSet::Chip8(InstructionSetChip8::Addi {
                    register: Register::from_repr(register).unwrap(),
                }),
                0x29 => Chip8InstructionSet::Chip8(InstructionSetChip8::Font {
                    register: Register::from_repr(register).unwrap(),
                }),
                0x33 => Chip8InstructionSet::Chip8(InstructionSetChip8::Bcd {
                    register: Register::from_repr(register).unwrap(),
                }),
                0x55 => Chip8InstructionSet::Chip8(InstructionSetChip8::Save { count: register }),
                0x65 => {
                    Chip8InstructionSet::Chip8(InstructionSetChip8::Restore { count: register })
                }
                _ => {
                    return None;
                }
            }
        }
        _ => {
            unreachable!()
        }
    })
}
