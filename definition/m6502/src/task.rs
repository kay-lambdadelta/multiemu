use crate::{
    ExecutionMode, LoadStep, M6502, M6502Config, RESET_VECTOR, StoreStep,
    decoder::M6502InstructionDecoder, instruction::AddressingMode, interpret::STACK_BASE_ADDRESS,
};
use arrayvec::ArrayVec;
use multiemu_machine::{
    component::RuntimeEssentials, processor::decoder::InstructionDecoder, scheduler::task::Task,
};
use std::{
    collections::VecDeque,
    num::NonZero,
    sync::{Arc, atomic::Ordering},
};

pub struct M6502Task {
    pub essentials: Arc<RuntimeEssentials>,
    pub instruction_decoder: M6502InstructionDecoder,
    pub config: Arc<M6502Config>,
}

impl Task<M6502> for M6502Task {
    fn run(&mut self, target: &M6502, period: NonZero<u32>) {
        let mut period = period.get();
        let mut state = target.state.lock().unwrap();

        while period != 0 {
            if !target.rdy.load(Ordering::Relaxed) {
                period -= 1;
                continue;
            }

            match state.execution_mode.take().unwrap() {
                ExecutionMode::Fetch => {
                    let (instruction, identifiying_bytes_length) = self
                        .instruction_decoder
                        .decode(
                            state.program as usize,
                            self.config.assigned_address_space,
                            self.essentials.memory_translation_table(),
                        )
                        .unwrap();

                    tracing::debug!("Decoded instruction: {:#?}", instruction);

                    state.address_bus =
                        state.program.wrapping_add(identifiying_bytes_length as u16);
                    state.program = state.program.wrapping_add(
                        identifiying_bytes_length as u16
                            + instruction
                                .addressing_mode
                                .map(|mode| mode.added_instruction_length())
                                .unwrap_or(0),
                    );

                    let latch = ArrayVec::new();

                    state.execution_mode = Some(match instruction.addressing_mode {
                        Some(AddressingMode::Absolute) => ExecutionMode::Load {
                            instruction,
                            latch,
                            queue: VecDeque::from_iter([
                                LoadStep::Data,
                                LoadStep::Data,
                                LoadStep::LatchToBus,
                            ]),
                        },
                        Some(AddressingMode::XIndexedAbsolute) => {
                            let offset = state.x;

                            ExecutionMode::Load {
                                instruction,
                                latch,
                                queue: VecDeque::from_iter([
                                    LoadStep::Data,
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                    LoadStep::Offset { offset },
                                ]),
                            }
                        }
                        Some(AddressingMode::YIndexedAbsolute) => {
                            let offset = state.y;

                            ExecutionMode::Load {
                                instruction,
                                latch,
                                queue: VecDeque::from_iter([
                                    LoadStep::Data,
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                    LoadStep::Offset { offset },
                                ]),
                            }
                        }
                        Some(AddressingMode::AbsoluteIndirect) => ExecutionMode::Load {
                            instruction,
                            latch,
                            queue: VecDeque::from_iter([
                                LoadStep::Data,
                                LoadStep::Data,
                                LoadStep::LatchToBus,
                                LoadStep::Data,
                                LoadStep::Data,
                                LoadStep::LatchToBus,
                            ]),
                        },
                        Some(AddressingMode::XIndexedZeroPageIndirect) => {
                            let offset = state.x;

                            ExecutionMode::Load {
                                instruction,
                                latch,
                                queue: VecDeque::from_iter([
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                    LoadStep::Offset { offset },
                                    LoadStep::Data,
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                ]),
                            }
                        }
                        Some(AddressingMode::ZeroPageIndirectYIndexed) => {
                            let offset = state.y;

                            ExecutionMode::Load {
                                instruction,
                                latch,
                                queue: VecDeque::from_iter([
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                    LoadStep::Data,
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                    LoadStep::Offset { offset },
                                ]),
                            }
                        }
                        Some(AddressingMode::XIndexedZeroPage) => {
                            let offset = state.x;

                            ExecutionMode::Load {
                                instruction,
                                latch,
                                queue: VecDeque::from_iter([
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                    LoadStep::Offset { offset },
                                ]),
                            }
                        }
                        Some(AddressingMode::YIndexedZeroPage) => {
                            let offset = state.y;

                            ExecutionMode::Load {
                                instruction,
                                latch,
                                queue: VecDeque::from_iter([
                                    LoadStep::Data,
                                    LoadStep::LatchToBus,
                                    LoadStep::Offset { offset },
                                ]),
                            }
                        }
                        Some(AddressingMode::ZeroPage) => ExecutionMode::Load {
                            instruction,
                            latch,
                            queue: VecDeque::from_iter([LoadStep::Data, LoadStep::LatchToBus]),
                        },
                        Some(AddressingMode::Relative)
                        | Some(AddressingMode::Immediate)
                        | Some(AddressingMode::Accumulator)
                        | None => ExecutionMode::Execute { instruction },
                    });

                    period -= 1;
                }
                ExecutionMode::Load {
                    instruction,
                    mut latch,
                    mut queue,
                } => {
                    match queue.pop_front() {
                        Some(LoadStep::Data) => {
                            let byte = self
                                .essentials
                                .memory_translation_table()
                                .read_le_value(
                                    state.address_bus as usize,
                                    self.config.assigned_address_space,
                                )
                                .unwrap_or_default();

                            latch.push(byte);
                            state.address_bus = state.address_bus.wrapping_add(1);

                            period -= 1;
                        }
                        Some(LoadStep::LatchToBus) => {
                            match latch.len() {
                                0 => {
                                    unreachable!()
                                }
                                1 => {
                                    state.address_bus = latch[0] as u16;
                                }
                                2 => {
                                    let latch = [latch[0], latch[1]];
                                    state.address_bus = u16::from_le_bytes(latch);
                                }
                                _ => {
                                    unreachable!()
                                }
                            }

                            latch.clear();
                        }
                        Some(LoadStep::Offset { offset }) => {
                            state.address_bus = state.address_bus.wrapping_add(offset as u16);
                        }
                        _ => unreachable!(),
                    }

                    if queue.is_empty() {
                        state.execution_mode = Some(ExecutionMode::Execute { instruction });
                    } else {
                        state.execution_mode = Some(ExecutionMode::Load {
                            instruction,
                            latch,
                            queue,
                        });
                    }
                }
                ExecutionMode::Jammed => {
                    period -= 1;
                }
                ExecutionMode::Reset => {
                    let program = [
                        self.essentials
                            .memory_translation_table()
                            .read_le_value(RESET_VECTOR, self.config.assigned_address_space)
                            .unwrap_or_default(),
                        self.essentials
                            .memory_translation_table()
                            .read_le_value(RESET_VECTOR + 1, self.config.assigned_address_space)
                            .unwrap_or_default(),
                    ];

                    state.program = u16::from_le_bytes(program);
                    state.execution_mode = Some(ExecutionMode::Fetch);

                    period -= 1;
                }
                ExecutionMode::Store { mut queue } => {
                    match queue.pop_front() {
                        Some(StoreStep::BusToProgram) => {
                            state.program = state.address_bus;
                            period -= 1;
                        }
                        Some(StoreStep::Data { value }) => {
                            let _ = self.essentials.memory_translation_table().write_le_value(
                                state.address_bus as usize,
                                self.config.assigned_address_space,
                                value,
                            );
                            state.address_bus = state.address_bus.wrapping_add(1);

                            period -= 1;
                        }
                        Some(StoreStep::PushStack { data }) => {
                            state.stack = state.stack.wrapping_sub(1);
                            let _ = self.essentials.memory_translation_table().write_le_value(
                                STACK_BASE_ADDRESS + state.stack as usize,
                                self.config.assigned_address_space,
                                data,
                            );

                            period -= 1;
                        }
                        Some(StoreStep::AddToProgram { value }) => {
                            state.program = state.program.wrapping_add_signed(value as i16);

                            period -= 1;
                        }
                        _ => {
                            unreachable!()
                        }
                    }

                    if queue.is_empty() {
                        state.execution_mode = Some(ExecutionMode::Fetch);
                    } else {
                        state.execution_mode = Some(ExecutionMode::Store { queue });
                    }
                }
                ExecutionMode::Execute { instruction } => {
                    state.execution_mode = Some(ExecutionMode::Fetch);

                    self.interpret_instruction(&mut state, instruction.clone());

                    period -= 1;
                }
            }
        }
    }
}
