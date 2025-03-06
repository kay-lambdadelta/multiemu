use super::{
    ExecutionState, ProcessorState,
    input::Chip8KeyCode,
    instruction::{Chip8InstructionSet, InstructionSetChip8},
    task::Chip8ProcessorTask,
};
use crate::{CHIP8_FONT, CPU_ADDRESS_SPACE, Chip8Kind};
use arrayvec::ArrayVec;
use bitvec::field::BitField;
use bitvec::prelude::{Lsb0, Msb0};
use bitvec::view::BitView;
use nalgebra::Point2;
use rand::Rng;
use std::sync::atomic::Ordering;

impl Chip8ProcessorTask {
    pub(super) fn interpret_instruction(
        &self,
        state: &mut ProcessorState,
        instruction: Chip8InstructionSet,
    ) {
        match instruction {
            Chip8InstructionSet::Chip8(InstructionSetChip8::Sys { syscall }) => match syscall {
                0x0e0 => {
                    self.display_component.interact(|component| {
                        component.clear_display();
                    });
                }
                0x0ee => {
                    if let Some(address) = state.stack.pop() {
                        state.registers.program = address;
                    } else {
                        tracing::error!("Stack underflow");
                        state.registers.program = 0x200;
                    }
                }
                _ => {
                    tracing::warn!("Unknown syscall: {:#04x}", syscall);
                }
            },
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
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skre {
                param_register_1,
                param_register_2,
            }) => {
                let param_register_1_value =
                    state.registers.work_registers[param_register_1 as usize];
                let param_register_2_value =
                    state.registers.work_registers[param_register_2 as usize];

                if param_register_1_value == param_register_2_value {
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
            Chip8InstructionSet::Chip8(InstructionSetChip8::Move {
                param_register_1,
                param_register_2,
            }) => {
                state.registers.work_registers[param_register_1 as usize] =
                    state.registers.work_registers[param_register_2 as usize];
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Or {
                destination,
                source,
            }) => {
                state.registers.work_registers[destination as usize] |=
                    state.registers.work_registers[source as usize];

                if self.mode.load() == Chip8Kind::Chip8 {
                    state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::And {
                destination,
                source,
            }) => {
                state.registers.work_registers[destination as usize] &=
                    state.registers.work_registers[source as usize];

                if self.mode.load() == Chip8Kind::Chip8 {
                    state.registers.work_registers[0xf] = 0;
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Xor {
                destination,
                source,
            }) => {
                state.registers.work_registers[destination as usize] ^=
                    state.registers.work_registers[source as usize];

                if self.mode.load() == Chip8Kind::Chip8 {
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
                state.registers.work_registers[0xf] = carry as u8;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Sub {
                destination,
                source,
            }) => {
                let destination_value = state.registers.work_registers[destination as usize];
                let source_value = state.registers.work_registers[source as usize];

                let (new_value, borrow) = destination_value.overflowing_sub(source_value);

                state.registers.work_registers[destination as usize] = new_value;
                state.registers.work_registers[0xf] = !borrow as u8;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Shr { register, value }) => {
                let mut destination_value = state.registers.work_registers[register as usize];

                if self.mode.load() == Chip8Kind::Chip8 || self.quirks.always_shr_in_place {
                    destination_value = state.registers.work_registers[value as usize];
                }

                let overflow = destination_value.view_bits::<Lsb0>()[0];

                state.registers.work_registers[register as usize] = destination_value >> 1;
                state.registers.work_registers[0xf] = overflow as u8;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Subn {
                destination,
                source,
            }) => {
                let destination_value = state.registers.work_registers[destination as usize];
                let source_value = state.registers.work_registers[source as usize];

                let (new_value, borrow) = source_value.overflowing_sub(destination_value);

                state.registers.work_registers[destination as usize] = new_value;
                state.registers.work_registers[0xf] = !borrow as u8;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Shl { register, value }) => {
                let mut destination_value = state.registers.work_registers[register as usize];

                if self.mode.load() == Chip8Kind::Chip8 {
                    destination_value = state.registers.work_registers[value as usize];
                }

                let overflow = destination_value.view_bits::<Lsb0>()[7];

                state.registers.work_registers[register as usize] = destination_value << 1;
                state.registers.work_registers[0xf] = overflow as u8;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Skrne {
                param_register_1,
                param_register_2,
            }) => {
                let param_register_1_value =
                    state.registers.work_registers[param_register_1 as usize];
                let param_register_2_value =
                    state.registers.work_registers[param_register_2 as usize];

                if param_register_1_value != param_register_2_value {
                    state.registers.program = state.registers.program.wrapping_add(2);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadi { value }) => {
                state.registers.index = value;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Jumpi { address }) => {
                let address = if self.mode.load() == Chip8Kind::Chip8 {
                    address.wrapping_add(state.registers.work_registers[0x0] as u16)
                } else {
                    let register = address.view_bits::<Msb0>()[4..8].load::<u8>();

                    address.wrapping_add(state.registers.work_registers[register as usize] as u16)
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
                coordinate_registers,
                height,
            }) => {
                let mut buffer =
                    ArrayVec::<_, 16>::from_iter(std::iter::repeat(0).take(height as usize));

                let mut cursor = 0;
                for buffer_section in buffer.chunks_mut(2) {
                    self.essentials
                        .memory_translation_table()
                        .read(
                            state.registers.index as usize + cursor,
                            CPU_ADDRESS_SPACE,
                            buffer_section,
                        )
                        .unwrap();
                    cursor += buffer_section.len();
                }

                let position = Point2::new(
                    state.registers.work_registers[coordinate_registers.x as usize],
                    state.registers.work_registers[coordinate_registers.y as usize],
                );

                self.display_component.interact(|display_component| {
                    state.registers.work_registers[0xf] =
                        display_component.draw_sprite(position, &buffer) as u8;
                });
                state.execution_state = ExecutionState::AwaitingVsync;
                self.vsync_occurred.store(false, Ordering::Relaxed);
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
                let mut delay_timer_value = 0;

                self.timer_component.interact(|timer_component| {
                    delay_timer_value = timer_component.get();
                });

                state.registers.work_registers[register as usize] = delay_timer_value;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Keyd { key: register }) => {
                state.execution_state = ExecutionState::AwaitingKeyPress { register };
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadd { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                self.timer_component.interact(|timer_component| {
                    timer_component.set(register_value);
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loads { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                self.audio_component.interact(|audio_component| {
                    audio_component.set(register_value);
                });
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Addi { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                state.registers.index = state.registers.index.wrapping_add(register_value as u16);
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Font { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                state.registers.index = register_value as u16 * CHIP8_FONT[0].len() as u16;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Bcd { register }) => {
                let register_value = state.registers.work_registers[register as usize];
                let [hundreds, tens, ones] = bcd_encode(register_value);
                let memory_translation_table = self.essentials.memory_translation_table();

                memory_translation_table
                    .write(
                        state.registers.index as usize,
                        CPU_ADDRESS_SPACE,
                        std::array::from_ref(&hundreds),
                    )
                    .unwrap();
                memory_translation_table
                    .write(
                        state.registers.index as usize + 1,
                        CPU_ADDRESS_SPACE,
                        std::array::from_ref(&tens),
                    )
                    .unwrap();
                memory_translation_table
                    .write(
                        state.registers.index as usize + 2,
                        CPU_ADDRESS_SPACE,
                        std::array::from_ref(&ones),
                    )
                    .unwrap();
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Save { count }) => {
                let memory_translation_table = self.essentials.memory_translation_table();

                for i in 0..=count {
                    memory_translation_table
                        .write(
                            state.registers.index as usize + i as usize,
                            CPU_ADDRESS_SPACE,
                            &state.registers.work_registers[i as usize..=i as usize],
                        )
                        .unwrap();
                }

                // Only the original chip8 modifies the index register for this operation
                if self.mode.load() == Chip8Kind::Chip8 {
                    state.registers.index = state.registers.index.wrapping_add(count as u16 + 1);
                }
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Restore { count }) => {
                let memory_translation_table = self.essentials.memory_translation_table();

                for i in 0..=count {
                    memory_translation_table
                        .read(
                            state.registers.index as usize + i as usize,
                            CPU_ADDRESS_SPACE,
                            &mut state.registers.work_registers[i as usize..=i as usize],
                        )
                        .unwrap();
                }

                // Only the original chip8 modifies the index register for this operation
                if self.mode.load() == Chip8Kind::Chip8 {
                    state.registers.index = state.registers.index.wrapping_add(count as u16 + 1);
                }
            }
            Chip8InstructionSet::SuperChip8(_) => todo!(),
            Chip8InstructionSet::XoChip(_) => todo!(),
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
