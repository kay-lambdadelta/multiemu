use crate::{
    ExecutionMode, LoadStep, Mos6502Config, ProcessorState, RESET_VECTOR, StoreStep,
    decoder::Mos6502InstructionDecoder,
    instruction::{AddressingMode, Mos6502AddressingMode, Wdc65C02AddressingMode},
    interpret::STACK_BASE_ADDRESS,
};
use arrayvec::ArrayVec;
use multiemu_runtime::{
    memory::memory_translation_table::MemoryTranslationTable,
    processor::decoder::InstructionDecoder,
    scheduler::{SchedulerHandle, Task, YieldReason},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Mos6502Task {
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    pub instruction_decoder: Mos6502InstructionDecoder,
    pub state: Arc<Mutex<ProcessorState>>,
    pub rdy: Arc<AtomicBool>,
    pub config: Arc<Mos6502Config>,
}

impl Task for Mos6502Task {
    fn run(self: Box<Self>, mut handle: SchedulerHandle) {
        // Keep the guard like this so doing a full load of the guard only happens occasionally
        let mut state_guard = Some(self.state.lock().unwrap());
        let mut should_exit = false;

        while !should_exit {
            if !self.rdy.load(Ordering::Relaxed) {
                tick_handler(&mut should_exit, &mut state_guard, &mut handle);

                continue;
            }

            let state = state_guard.get_or_insert_with(|| self.state.lock().unwrap());

            match state.execution_mode.take().unwrap() {
                ExecutionMode::FetchAndDecode => {
                    tracing::debug!(
                        "Fetching and decoding instruction from {:#04x}",
                        state.program
                    );

                    fetch_and_decode(
                        state,
                        &self.instruction_decoder,
                        &self.config,
                        &self.memory_translation_table,
                    );

                    tick_handler(&mut should_exit, &mut state_guard, &mut handle);
                }
                ExecutionMode::PreInterpret {
                    instruction,
                    mut latch,
                    mut queue,
                } => {
                    let mut tick = false;

                    match queue.pop_front() {
                        Some(LoadStep::Data) => {
                            let byte = self
                                .memory_translation_table
                                .read_le_value(
                                    state.address_bus as usize,
                                    self.config.assigned_address_space,
                                )
                                .unwrap_or_default();

                            latch.push(byte);
                            state.address_bus = state.address_bus.wrapping_add(1);

                            tick = true;
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
                        state.execution_mode = Some(ExecutionMode::Interpret { instruction });
                    } else {
                        state.execution_mode = Some(ExecutionMode::PreInterpret {
                            instruction,
                            latch,
                            queue,
                        });
                    }

                    if tick {
                        tick_handler(&mut should_exit, &mut state_guard, &mut handle);
                    }
                }
                ExecutionMode::Jammed => {
                    tick_handler(&mut should_exit, &mut state_guard, &mut handle);
                }
                ExecutionMode::Wait => {
                    tick_handler(&mut should_exit, &mut state_guard, &mut handle);
                }
                ExecutionMode::Reset => {
                    let program = [
                        self.memory_translation_table
                            .read_le_value(RESET_VECTOR, self.config.assigned_address_space)
                            .unwrap_or_default(),
                        self.memory_translation_table
                            .read_le_value(RESET_VECTOR + 1, self.config.assigned_address_space)
                            .unwrap_or_default(),
                    ];

                    state.program = u16::from_le_bytes(program);
                    state.execution_mode = Some(ExecutionMode::FetchAndDecode);

                    tick_handler(&mut should_exit, &mut state_guard, &mut handle);
                }
                ExecutionMode::PostInterpret { mut queue } => {
                    match queue.pop_front() {
                        Some(StoreStep::BusToProgram) => {
                            state.program = state.address_bus;
                        }
                        Some(StoreStep::Data { value }) => {
                            let _ = self.memory_translation_table.write_le_value(
                                state.address_bus as usize,
                                self.config.assigned_address_space,
                                value,
                            );
                            state.address_bus = state.address_bus.wrapping_add(1);
                        }
                        Some(StoreStep::PushStack { data }) => {
                            state.stack = state.stack.wrapping_sub(1);
                            let _ = self.memory_translation_table.write_le_value(
                                STACK_BASE_ADDRESS + state.stack as usize,
                                self.config.assigned_address_space,
                                data,
                            );
                        }
                        Some(StoreStep::AddToProgram { value }) => {
                            state.program = state.program.wrapping_add_signed(value as i16);
                        }
                        _ => {
                            unreachable!()
                        }
                    }

                    if queue.is_empty() {
                        state.execution_mode = Some(ExecutionMode::FetchAndDecode);
                    } else {
                        state.execution_mode = Some(ExecutionMode::PostInterpret { queue });
                    }

                    tick_handler(&mut should_exit, &mut state_guard, &mut handle);
                }
                ExecutionMode::Interpret { instruction } => {
                    state.execution_mode = Some(ExecutionMode::FetchAndDecode);

                    self.interpret_instruction(state, instruction.clone());

                    tick_handler(&mut should_exit, &mut state_guard, &mut handle);
                }
            }
        }
    }
}

#[inline]
fn fetch_and_decode(
    state: &mut ProcessorState,
    instruction_decoder: &Mos6502InstructionDecoder,
    config: &Mos6502Config,
    memory_translation_table: &MemoryTranslationTable,
) {
    let (instruction, identifiying_bytes_length) = instruction_decoder
        .decode(
            state.program as usize,
            config.assigned_address_space,
            memory_translation_table,
        )
        .unwrap();

    debug_assert!(
        instruction
            .addressing_mode
            .map(|addressing_mode| { addressing_mode.is_valid_for_mode(config.kind) })
            .unwrap_or(true),
        "Invalid addressing mode for instruction for mode {:?}: {:?}",
        config.kind,
        instruction,
    );

    tracing::debug!("Decoded instruction: {:#?}", instruction);

    state.address_bus = state.program.wrapping_add(identifiying_bytes_length as u16);
    state.program = state.program.wrapping_add(
        identifiying_bytes_length as u16
            + instruction
                .addressing_mode
                .map(|mode| mode.added_instruction_length())
                .unwrap_or(0),
    );

    let latch = ArrayVec::new();

    state.execution_mode = Some(match instruction.addressing_mode {
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::Absolute)) => {
            ExecutionMode::PreInterpret {
                instruction,
                latch,
                queue: VecDeque::from_iter([LoadStep::Data, LoadStep::Data, LoadStep::LatchToBus]),
            }
        }
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute)) => {
            let offset = state.x;

            ExecutionMode::PreInterpret {
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
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute)) => {
            let offset = state.y;

            ExecutionMode::PreInterpret {
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
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::AbsoluteIndirect)) => {
            ExecutionMode::PreInterpret {
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
            }
        }
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPageIndirect)) => {
            let offset = state.x;

            ExecutionMode::PreInterpret {
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
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPageIndirectYIndexed)) => {
            let offset = state.y;

            ExecutionMode::PreInterpret {
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
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage)) => {
            let offset = state.x;

            ExecutionMode::PreInterpret {
                instruction,
                latch,
                queue: VecDeque::from_iter([
                    LoadStep::Data,
                    LoadStep::LatchToBus,
                    LoadStep::Offset { offset },
                ]),
            }
        }
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedZeroPage)) => {
            let offset = state.y;

            ExecutionMode::PreInterpret {
                instruction,
                latch,
                queue: VecDeque::from_iter([
                    LoadStep::Data,
                    LoadStep::LatchToBus,
                    LoadStep::Offset { offset },
                ]),
            }
        }
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPage)) => {
            ExecutionMode::PreInterpret {
                instruction,
                latch,
                queue: VecDeque::from_iter([LoadStep::Data, LoadStep::LatchToBus]),
            }
        }
        Some(AddressingMode::Wdc65C02(Wdc65C02AddressingMode::ZeroPageIndirect)) => {
            todo!()
        }
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::Relative))
        | Some(AddressingMode::Mos6502(Mos6502AddressingMode::Immediate))
        | Some(AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator))
        | None => ExecutionMode::Interpret { instruction },
    });
}

#[inline]
fn tick_handler(
    should_exit: &mut bool,
    state_guard: &mut Option<MutexGuard<ProcessorState>>,
    handle: &mut SchedulerHandle,
) {
    handle.tick(|reason| match reason {
        YieldReason::Exit => {
            *should_exit = true;
        }
        YieldReason::TimeSynchronization => {}
        YieldReason::RuntimeInterrupt => {
            // Release the guard here
            state_guard.take();
        }
    });
}
