use super::{
    ExecutionState, ProcessorState,
    input::Chip8KeyCode,
    instruction::{Chip8InstructionSet, InstructionSetChip8},
    task::Chip8ProcessorTask,
};
use crate::{CHIP8_FONT, Chip8Kind, processor::instruction::InstructionSetSuperChip8};
use arrayvec::ArrayVec;
use bitvec::{
    field::BitField,
    prelude::{Lsb0, Msb0},
    view::BitView,
};
use nalgebra::Point2;
use rand::Rng;

// Instruction interpreting can be clean and easy due to the chip8 enforcing 1 cycle = 1 instruction

impl Chip8ProcessorTask {
    pub(super) fn interpret_instruction(
        &self,
        state: &mut ProcessorState,
        instruction: Chip8InstructionSet,
    ) {
        tracing::trace!("Interpreting instruction: {:x?}", instruction);

        match instruction {
            Chip8InstructionSet::Chip8(InstructionSetChip8::Clr) => {
                self.config
                    .display
                    .interact(|component| {
                        component.clear_display();
                    })
                    .unwrap();
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
                coordinates: coordinate_registers,
                height,
            }) => {
                let mut buffer =
                    ArrayVec::<_, 16>::from_iter(std::iter::repeat_n(0, height as usize));

                let mut cursor = 0;
                for buffer_section in buffer.chunks_mut(2) {
                    self.essentials
                        .memory_translation_table
                        .read(
                            state.registers.index as usize + cursor,
                            self.config.cpu_address_space,
                            buffer_section,
                        )
                        .unwrap();
                    cursor += buffer_section.len();
                }

                let position = Point2::new(
                    state.registers.work_registers[coordinate_registers.x as usize],
                    state.registers.work_registers[coordinate_registers.y as usize],
                );

                self.config
                    .display
                    .interact(|display_component| {
                        state.registers.work_registers[0xf] =
                            display_component.draw_sprite(position, &buffer) as u8;
                    })
                    .unwrap();
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
                let mut delay_timer_value = 0;
                self.config
                    .timer
                    .interact(|timer_component| {
                        delay_timer_value = timer_component.get();
                    })
                    .unwrap();

                state.registers.work_registers[register as usize] = delay_timer_value;
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Keyd { key: register }) => {
                state.execution_state = ExecutionState::AwaitingKeyPress { register };
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loadd { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                self.config
                    .timer
                    .interact(|timer_component| {
                        timer_component.set(register_value);
                    })
                    .unwrap();
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Loads { register }) => {
                let register_value = state.registers.work_registers[register as usize];

                self.config
                    .audio
                    .interact(|audio_component| {
                        audio_component.set(register_value);
                    })
                    .unwrap();
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

                self.essentials
                    .memory_translation_table
                    .write_le_value(
                        state.registers.index as usize,
                        self.config.cpu_address_space,
                        hundreds,
                    )
                    .unwrap();
                self.essentials
                    .memory_translation_table
                    .write_le_value(
                        state.registers.index as usize + 1,
                        self.config.cpu_address_space,
                        tens,
                    )
                    .unwrap();
                self.essentials
                    .memory_translation_table
                    .write_le_value(
                        state.registers.index as usize + 2,
                        self.config.cpu_address_space,
                        ones,
                    )
                    .unwrap();
            }
            Chip8InstructionSet::Chip8(InstructionSetChip8::Save { count }) => {
                for i in 0..=count {
                    self.essentials
                        .memory_translation_table
                        .write(
                            state.registers.index as usize + i as usize,
                            self.config.cpu_address_space,
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
                for i in 0..=count {
                    self.essentials
                        .memory_translation_table
                        .read(
                            state.registers.index as usize + i as usize,
                            self.config.cpu_address_space,
                            &mut state.registers.work_registers[i as usize..=i as usize],
                        )
                        .unwrap();
                }

                // Only the original chip8 modifies the index register for this operation
                if self.mode.load() == Chip8Kind::Chip8 {
                    state.registers.index = state.registers.index.wrapping_add(count as u16 + 1);
                }
            }
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Hires) => {}
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Lores) => {}
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Scroll { direction }) => {}
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Scrd { amount }) => {}
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Scrr) => {}
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Scrl) => {}
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Srpl { amount }) => {}
            Chip8InstructionSet::SuperChip8(InstructionSetSuperChip8::Rrpl { amount }) => {}
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
