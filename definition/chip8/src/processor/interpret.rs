use arrayvec::ArrayVec;
use bitvec::{
    field::BitField,
    prelude::{Lsb0, Msb0},
    view::BitView,
};
use nalgebra::Point2;
use rand::Rng;

use super::{
    ExecutionState,
    input::Chip8KeyCode,
    instruction::{Chip8InstructionSet, InstructionSetChip8},
};
use crate::{
    CHIP8_FONT, Chip8Mode,
    display::SupportedGraphicsApiChip8Display,
    processor::{Chip8Processor, instruction::InstructionSetSuperChip8},
};

// Instruction interpreting can be clean and easy due to the chip8 enforcing 1
// cycle = 1 instruction

impl<G: SupportedGraphicsApiChip8Display> Chip8Processor<G> {
    pub(super) fn interpret_instruction(&mut self, instruction: Chip8InstructionSet) {
        let mut mode_guard = self.mode.lock().unwrap();

        match instruction {
            Chip8InstructionSet::Chip8(InstructionSetChip8::Clr) => {
                self.display.interact_mut(self.timestamp, |component| {
                    component.clear_display();
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Rtrn) => {
                if let Some(address) = self.state.stack.pop() {
                    self.state.registers.program = address;
                } else {
                    tracing::error!("Stack underflow");
                    self.state.registers.program = 0x200;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Jump { address }) => {
                self.state.registers.program = address;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Call { address }) => {
                let program = self.state.registers.program;
                self.state.stack.push(program);
                self.state.registers.program = address;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Ske {
                register,
                immediate,
            }) => {
                let register_value = self.state.registers.work_registers[register as usize];

                if register_value == immediate {
                    self.state.registers.program = self.state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skne {
                register,
                immediate,
            }) => {
                let register_value = self.state.registers.work_registers[register as usize];

                if register_value != immediate {
                    self.state.registers.program = self.state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skre { param_1, param_2 }) => {
                let param_1_value = self.state.registers.work_registers[param_1 as usize];
                let param_2_value = self.state.registers.work_registers[param_2 as usize];

                if param_1_value == param_2_value {
                    self.state.registers.program = self.state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Load {
                register,
                immediate,
            }) => {
                self.state.registers.work_registers[register as usize] = immediate;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Add {
                register,
                immediate,
            }) => {
                let register_value = self.state.registers.work_registers[register as usize];

                self.state.registers.work_registers[register as usize] =
                    register_value.wrapping_add(immediate);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Move { param_1, param_2 }) => {
                self.state.registers.work_registers[param_1 as usize] =
                    self.state.registers.work_registers[param_2 as usize];
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Or {
                destination,
                source,
            }) => {
                self.state.registers.work_registers[destination as usize] |=
                    self.state.registers.work_registers[source as usize];

                if *mode_guard == Chip8Mode::Chip8 {
                    self.state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::And {
                destination,
                source,
            }) => {
                self.state.registers.work_registers[destination as usize] &=
                    self.state.registers.work_registers[source as usize];

                if *mode_guard == Chip8Mode::Chip8 {
                    self.state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Xor {
                destination,
                source,
            }) => {
                self.state.registers.work_registers[destination as usize] ^=
                    self.state.registers.work_registers[source as usize];

                if *mode_guard == Chip8Mode::Chip8 {
                    self.state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Addr {
                destination,
                source,
            }) => {
                let destination_value = self.state.registers.work_registers[destination as usize];
                let source_value = self.state.registers.work_registers[source as usize];

                let (new_value, carry) = destination_value.overflowing_add(source_value);

                self.state.registers.work_registers[destination as usize] = new_value;
                self.state.registers.work_registers[0xf] = u8::from(carry);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Sub {
                destination,
                source,
            }) => {
                let destination_value = self.state.registers.work_registers[destination as usize];
                let source_value = self.state.registers.work_registers[source as usize];

                let (new_value, borrow) = destination_value.overflowing_sub(source_value);

                self.state.registers.work_registers[destination as usize] = new_value;
                self.state.registers.work_registers[0xf] = u8::from(!borrow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Shr { register, value }) => {
                let mut destination_value = self.state.registers.work_registers[register as usize];

                if *mode_guard == Chip8Mode::Chip8 || self.config.always_shr_in_place {
                    destination_value = self.state.registers.work_registers[value as usize];
                }

                let overflow = destination_value.view_bits::<Lsb0>()[0];

                self.state.registers.work_registers[register as usize] = destination_value >> 1;
                self.state.registers.work_registers[0xf] = u8::from(overflow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Subn {
                destination,
                source,
            }) => {
                let destination_value = self.state.registers.work_registers[destination as usize];
                let source_value = self.state.registers.work_registers[source as usize];

                let (new_value, borrow) = source_value.overflowing_sub(destination_value);

                self.state.registers.work_registers[destination as usize] = new_value;
                self.state.registers.work_registers[0xf] = u8::from(!borrow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Shl { register, value }) => {
                let mut destination_value = self.state.registers.work_registers[register as usize];

                if *mode_guard == Chip8Mode::Chip8 {
                    destination_value = self.state.registers.work_registers[value as usize];
                }

                let overflow = destination_value.view_bits::<Lsb0>()[7];

                self.state.registers.work_registers[register as usize] = destination_value << 1;
                self.state.registers.work_registers[0xf] = u8::from(overflow);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skrne { param_1, param_2 }) => {
                let param_1_value = self.state.registers.work_registers[param_1 as usize];
                let param_2_value = self.state.registers.work_registers[param_2 as usize];

                if param_1_value != param_2_value {
                    self.state.registers.program = self.state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadi { value }) => {
                self.state.registers.index = value;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Jumpi { address }) => {
                let address = if *mode_guard == Chip8Mode::Chip8 {
                    address.wrapping_add(u16::from(self.state.registers.work_registers[0x0]))
                } else {
                    let register = address.view_bits::<Msb0>()[4..8].load::<u8>();

                    address.wrapping_add(u16::from(
                        self.state.registers.work_registers[register as usize],
                    ))
                };

                self.state.registers.program = address;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Rand {
                register,
                immediate,
            }) => {
                self.state.registers.work_registers[register as usize] =
                    rand::rng().random::<u8>() & immediate;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Draw {
                coordinates,
                height,
            }) => {
                let position = Point2::new(
                    self.state.registers.work_registers[coordinates.x as usize],
                    self.state.registers.work_registers[coordinates.y as usize],
                );
                let mut cursor = 0;

                // SuperChip8 specializes a 16x16 sprite here
                if height == 0 && *mode_guard == Chip8Mode::SuperChip8
                    || *mode_guard == Chip8Mode::XoChip
                {
                    let mut buffer = [0; 32];

                    for buffer_section in buffer.chunks_mut(2) {
                        self.cpu_address_space
                            .read(
                                self.state.registers.index as usize + cursor,
                                self.timestamp,
                                None,
                                buffer_section,
                            )
                            .unwrap();
                        cursor += buffer_section.len();
                    }

                    self.state.registers.work_registers[0xf] =
                        self.display.interact_mut(self.timestamp, |component| {
                            u8::from(component.draw_supersized_sprite(position, buffer))
                        });
                } else {
                    let mut buffer =
                        ArrayVec::<_, 16>::from_iter(std::iter::repeat_n(0, height as usize));

                    for buffer_section in buffer.chunks_mut(2) {
                        self.cpu_address_space
                            .read(
                                self.state.registers.index as usize + cursor,
                                self.timestamp,
                                None,
                                buffer_section,
                            )
                            .unwrap();
                        cursor += buffer_section.len();
                    }

                    self.state.registers.work_registers[0xf] =
                        self.display.interact_mut(self.timestamp, |component| {
                            u8::from(component.draw_sprite(position, &buffer))
                        });
                }

                self.state.execution_state = ExecutionState::AwaitingVsync;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skpr { key }) => {
                let key = Chip8KeyCode(self.state.registers.work_registers[key as usize]);

                let key_value = self.virtual_gamepad.get(key.try_into().unwrap());

                if key_value.as_digital(None) {
                    self.state.registers.program = self.state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skup { key }) => {
                let key = Chip8KeyCode(self.state.registers.work_registers[key as usize]);

                let key_value = self.virtual_gamepad.get(key.try_into().unwrap());

                if !key_value.as_digital(None) {
                    self.state.registers.program = self.state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Moved { register }) => {
                self.state.registers.work_registers[register as usize] = self
                    .timer
                    .interact(self.timestamp, |component| component.get());
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Keyd { key: register }) => {
                self.state.execution_state = ExecutionState::AwaitingKeyPress { register };
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadd { register }) => {
                let register_value = self.state.registers.work_registers[register as usize];

                self.timer.interact_mut(self.timestamp, |component| {
                    component.set(register_value);
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loads { register }) => {
                let register_value = self.state.registers.work_registers[register as usize];

                self.audio.interact_mut(self.timestamp, |component| {
                    component.set(register_value);
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Addi { register }) => {
                let register_value = self.state.registers.work_registers[register as usize];

                self.state.registers.index = self
                    .state
                    .registers
                    .index
                    .wrapping_add(u16::from(register_value));
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Font { register }) => {
                let register_value = self.state.registers.work_registers[register as usize];

                self.state.registers.index = u16::from(register_value) * CHIP8_FONT[0].len() as u16;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Bcd { register }) => {
                let register_value = self.state.registers.work_registers[register as usize];
                let [hundreds, tens, ones] = bcd_encode(register_value);

                self.cpu_address_space
                    .write_le_value(
                        self.state.registers.index as usize,
                        self.timestamp,
                        None,
                        hundreds,
                    )
                    .unwrap();
                self.cpu_address_space
                    .write_le_value(
                        self.state.registers.index as usize + 1,
                        self.timestamp,
                        None,
                        tens,
                    )
                    .unwrap();
                self.cpu_address_space
                    .write_le_value(
                        self.state.registers.index as usize + 2,
                        self.timestamp,
                        None,
                        ones,
                    )
                    .unwrap();
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Save { count }) => {
                for i in 0..=count {
                    self.cpu_address_space
                        .write(
                            self.state.registers.index as usize + i as usize,
                            self.timestamp,
                            None,
                            &self.state.registers.work_registers[i as usize..=i as usize],
                        )
                        .unwrap();
                }

                // Only the original chip8 modifies the index register for this operation
                if *mode_guard == Chip8Mode::Chip8 {
                    self.state.registers.index = self
                        .state
                        .registers
                        .index
                        .wrapping_add(u16::from(count) + 1);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Restore { count }) => {
                for i in 0..=count {
                    self.cpu_address_space
                        .read(
                            self.state.registers.index as usize + i as usize,
                            self.timestamp,
                            None,
                            &mut self.state.registers.work_registers[i as usize..=i as usize],
                        )
                        .unwrap();
                }

                // Only the original chip8 modifies the index register for this operation
                if *mode_guard == Chip8Mode::Chip8 {
                    self.state.registers.index = self
                        .state
                        .registers
                        .index
                        .wrapping_add(u16::from(count) + 1);
                }
            }
            Chip8InstructionSet::SuperChip8(subinstruction) => {
                *mode_guard = Chip8Mode::SuperChip8;

                match subinstruction {
                    InstructionSetSuperChip8::Lores => {
                        self.display.interact_mut(self.timestamp, |component| {
                            component.set_hires(false);
                        });
                    }
                    InstructionSetSuperChip8::Hires => {
                        self.display.interact_mut(self.timestamp, |component| {
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
                *mode_guard = Chip8Mode::XoChip;

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
