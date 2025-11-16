use arrayvec::ArrayVec;
use bitvec::{
    field::BitField,
    prelude::{Lsb0, Msb0},
    view::BitView,
};
use nalgebra::Point2;
use rand::Rng;

use super::{
    ExecutionState, ProcessorState,
    input::Chip8KeyCode,
    instruction::{Chip8InstructionSet, InstructionSetChip8},
    task::Driver,
};
use crate::{
    CHIP8_FONT, Chip8Mode, display::SupportedGraphicsApiChip8Display,
    processor::instruction::InstructionSetSuperChip8,
};

// Instruction interpreting can be clean and easy due to the chip8 enforcing 1 cycle = 1 instruction

impl<G: SupportedGraphicsApiChip8Display> Driver<G> {
    pub(super) fn interpret_instruction(
        &self,
        state: &mut ProcessorState,
        mode: &mut Chip8Mode,
        instruction: Chip8InstructionSet,
    ) {
        tracing::trace!("Interpreting instruction: {:x?}", instruction);

        match instruction {
            Chip8InstructionSet::Chip8(InstructionSetChip8::Clr) => {
                self.display.interact_mut(|component| {
                    component.clear_display();
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Rtrn) => {
                if let Some(address) = state.stack.pop() {
                    state.registers.program = address;
                } else {
                    tracing::error!("Stack underflow");
                    state.registers.program = 0x200;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Jump { address }) => {
                state.registers.program = address;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Call { address }) => {
                let program = state.registers.program;
                state.stack.push(program);
                state.registers.program = address;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Ske {
                register,
                immediate,
            }) => {
                let register_value = state.registers.work_registers[register as usize];

                if register_value == immediate {
                    state.registers.program = state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skne {
                register,
                immediate,
            }) => {
                let register_value = state.registers.work_registers[register as usize];

                if register_value != immediate {
                    state.registers.program = state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skre { param_1, param_2 }) => {
                let param_1_value = state.registers.work_registers[param_1 as usize];
                let param_2_value = state.registers.work_registers[param_2 as usize];

                if param_1_value == param_2_value {
                    state.registers.program = state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Load {
                register,
                immediate,
            }) => {
                state.registers.work_registers[register as usize] = immediate;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Add {
                register,
                immediate,
            }) => {
                let register_value = state.registers.work_registers[register as usize];

                state.registers.work_registers[register as usize] =
                    register_value.wrapping_add(immediate);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Move { param_1, param_2 }) => {
                state.registers.work_registers[param_1 as usize] =
                    state.registers.work_registers[param_2 as usize];
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Or {
                destination,
                source,
            }) => {
                state.registers.work_registers[destination as usize] |=
                    state.registers.work_registers[source as usize];

                if *mode == Chip8Mode::Chip8 {
                    state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::And {
                destination,
                source,
            }) => {
                state.registers.work_registers[destination as usize] &=
                    state.registers.work_registers[source as usize];

                if *mode == Chip8Mode::Chip8 {
                    state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Xor {
                destination,
                source,
            }) => {
                state.registers.work_registers[destination as usize] ^=
                    state.registers.work_registers[source as usize];

                if *mode == Chip8Mode::Chip8 {
                    state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Addr {
                destination,
                source,
            }) => {
                let destination_value = state.registers.work_registers[destination as usize];
                let source_value = state.registers.work_registers[source as usize];

                let (new_value, carry) = destination_value.overflowing_add(source_value);

                state.registers.work_registers[destination as usize] = new_value;
                state.registers.work_registers[0xf] = u8::from(carry);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Sub {
                destination,
                source,
            }) => {
                let destination_value = state.registers.work_registers[destination as usize];
                let source_value = state.registers.work_registers[source as usize];

                let (new_value, borrow) = destination_value.overflowing_sub(source_value);

                state.registers.work_registers[destination as usize] = new_value;
                state.registers.work_registers[0xf] = u8::from(!borrow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Shr { register, value }) => {
                let mut destination_value = state.registers.work_registers[register as usize];

                if *mode == Chip8Mode::Chip8 || self.config.always_shr_in_place {
                    destination_value = state.registers.work_registers[value as usize];
                }

                let overflow = destination_value.view_bits::<Lsb0>()[0];

                state.registers.work_registers[register as usize] = destination_value >> 1;
                state.registers.work_registers[0xf] = u8::from(overflow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Subn {
                destination,
                source,
            }) => {
                let destination_value = state.registers.work_registers[destination as usize];
                let source_value = state.registers.work_registers[source as usize];

                let (new_value, borrow) = source_value.overflowing_sub(destination_value);

                state.registers.work_registers[destination as usize] = new_value;
                state.registers.work_registers[0xf] = u8::from(!borrow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Shl { register, value }) => {
                let mut destination_value = state.registers.work_registers[register as usize];

                if *mode == Chip8Mode::Chip8 {
                    destination_value = state.registers.work_registers[value as usize];
                }

                let overflow = destination_value.view_bits::<Lsb0>()[7];

                state.registers.work_registers[register as usize] = destination_value << 1;
                state.registers.work_registers[0xf] = u8::from(overflow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skrne { param_1, param_2 }) => {
                let param_1_value = state.registers.work_registers[param_1 as usize];
                let param_2_value = state.registers.work_registers[param_2 as usize];

                if param_1_value != param_2_value {
                    state.registers.program = state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadi { value }) => {
                state.registers.index = value;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Jumpi { address }) => {
                let address = if *mode == Chip8Mode::Chip8 {
                    address.wrapping_add(u16::from(state.registers.work_registers[0x0]))
                } else {
                    let register = address.view_bits::<Msb0>()[4..8].load::<u8>();

                    address
                        .wrapping_add(u16::from(state.registers.work_registers[register as usize]))
                };

                state.registers.program = address;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Rand {
                register,
                immediate,
            }) => {
                state.registers.work_registers[register as usize] =
                    rand::rng().random::<u8>() & immediate;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Draw {
                coordinates,
                height,
            }) => {
                let position = Point2::new(
                    state.registers.work_registers[coordinates.x as usize],
                    state.registers.work_registers[coordinates.y as usize],
                );
                let mut cursor = 0;

                // SuperChip8 specializes a 16x16 sprite here
                if height == 0 && *mode == Chip8Mode::SuperChip8 || *mode == Chip8Mode::XoChip {
                    let mut buffer = [0; 32];

                    for buffer_section in buffer.chunks_mut(2) {
                        self.cpu_address_space
                            .read(
                                state.registers.index as usize + cursor,
                                false,
                                buffer_section,
                            )
                            .unwrap();
                        cursor += buffer_section.len();
                    }

                    state.registers.work_registers[0xf] = self.display.interact_mut(|component| {
                        u8::from(component.draw_supersized_sprite(position, buffer))
                    });
                } else {
                    let mut buffer =
                        ArrayVec::<_, 16>::from_iter(std::iter::repeat_n(0, height as usize));

                    for buffer_section in buffer.chunks_mut(2) {
                        self.cpu_address_space
                            .read(
                                state.registers.index as usize + cursor,
                                false,
                                buffer_section,
                            )
                            .unwrap();
                        cursor += buffer_section.len();
                    }

                    state.registers.work_registers[0xf] = self.display.interact_mut(|component| {
                        u8::from(component.draw_sprite(position, &buffer))
                    });
                }

                state.execution_state = ExecutionState::AwaitingVsync;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skpr { key }) => {
                let key = Chip8KeyCode(state.registers.work_registers[key as usize]);

                let key_value = self.virtual_gamepad.get(key.try_into().unwrap());

                if key_value.as_digital(None) {
                    state.registers.program = state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skup { key }) => {
                let key = Chip8KeyCode(state.registers.work_registers[key as usize]);

                let key_value = self.virtual_gamepad.get(key.try_into().unwrap());

                if !key_value.as_digital(None) {
                    state.registers.program = state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Moved { register }) => {
                state.registers.work_registers[register as usize] =
                    self.timer.interact(|component| component.get());
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Keyd { key: register }) => {
                state.execution_state = ExecutionState::AwaitingKeyPress { register };
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadd { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                self.timer.interact_mut(|component| {
                    component.set(register_value);
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loads { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                self.audio.interact_mut(|component| {
                    component.set(register_value);
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Addi { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                state.registers.index = state
                    .registers
                    .index
                    .wrapping_add(u16::from(register_value));
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Font { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                state.registers.index = u16::from(register_value) * CHIP8_FONT[0].len() as u16;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Bcd { register }) => {
                let register_value = state.registers.work_registers[register as usize];
                let [hundreds, tens, ones] = bcd_encode(register_value);

                self.cpu_address_space
                    .write_le_value(state.registers.index as usize, hundreds)
                    .unwrap();
                self.cpu_address_space
                    .write_le_value(state.registers.index as usize + 1, tens)
                    .unwrap();
                self.cpu_address_space
                    .write_le_value(state.registers.index as usize + 2, ones)
                    .unwrap();
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Save { count }) => {
                for i in 0..=count {
                    self.cpu_address_space
                        .write(
                            state.registers.index as usize + i as usize,
                            &state.registers.work_registers[i as usize..=i as usize],
                        )
                        .unwrap();
                }

                // Only the original chip8 modifies the index register for this operation
                if *mode == Chip8Mode::Chip8 {
                    state.registers.index =
                        state.registers.index.wrapping_add(u16::from(count) + 1);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Restore { count }) => {
                for i in 0..=count {
                    self.cpu_address_space
                        .read(
                            state.registers.index as usize + i as usize,
                            false,
                            &mut state.registers.work_registers[i as usize..=i as usize],
                        )
                        .unwrap();
                }

                // Only the original chip8 modifies the index register for this operation
                if *mode == Chip8Mode::Chip8 {
                    state.registers.index =
                        state.registers.index.wrapping_add(u16::from(count) + 1);
                }
            }
            Chip8InstructionSet::SuperChip8(subinstruction) => {
                *mode = Chip8Mode::SuperChip8;

                match subinstruction {
                    InstructionSetSuperChip8::Lores => {
                        self.display.interact_mut(|component| {
                            component.set_hires(false);
                        });
                    }
                    InstructionSetSuperChip8::Hires => {
                        self.display.interact_mut(|component| {
                            component.set_hires(true);
                        });
                    }
                    InstructionSetSuperChip8::Scroll { direction } => todo!(),
                    InstructionSetSuperChip8::Scrd { amount } => todo!(),
                    InstructionSetSuperChip8::Scrr => todo!(),
                    InstructionSetSuperChip8::Scrl => todo!(),
                    InstructionSetSuperChip8::Srpl { amount } => todo!(),
                    InstructionSetSuperChip8::Rrpl { amount } => todo!(),
                }
            }
            Chip8InstructionSet::XoChip(_) => {
                *mode = Chip8Mode::XoChip;

                todo!()
            }
        }
    }
}

#[inline]
fn bcd_encode(value: u8) -> [u8; 3] {
    let hundreds = value / 100;
    let tens = (value / 10) % 10;
    let ones = value % 10;

    [hundreds, tens, ones]
}
