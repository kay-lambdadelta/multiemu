use crate::{
    ExecutionMode, LoadStep, Mos6502, Mos6502Config, PostInterpretStep, ProcessorState,
    RESET_VECTOR,
    decoder::Mos6502InstructionDecoder,
    instruction::{AddressingMode, Mos6502AddressingMode, Wdc65C02AddressingMode},
    interpret::STACK_BASE_ADDRESS,
};
use arrayvec::ArrayVec;
use multiemu_runtime::{
    component::ComponentRef, memory::MemoryAccessTable, processor::InstructionDecoder,
    scheduler::Task,
};
use std::{
    collections::VecDeque,
    num::NonZero,
    sync::{Arc, atomic::Ordering},
};

pub struct Mos6502Task {
    pub memory_translation_table: Arc<MemoryAccessTable>,
    pub instruction_decoder: Mos6502InstructionDecoder,
    pub component: ComponentRef<Mos6502>,
}

impl Task for Mos6502Task {
    fn run(&mut self, time_slice: NonZero<u32>) {
        self.component
            .interact(|component| {
                // Keep the guard like this so doing a full load of the guard only happens occasionally
                let mut state_guard = component.state.lock().unwrap();
                let mut time_slice = time_slice.get();

                while time_slice != 0 {
                    if !component.rdy.load(Ordering::Relaxed) {
                        time_slice -= 1;
                        continue;
                    }

                    match state_guard.execution_mode.take().unwrap() {
                        ExecutionMode::FetchAndDecode => {
                            tracing::debug!(
                                "Fetching and decoding instruction from {:#04x}",
                                state_guard.program
                            );

                            fetch_and_decode(
                                &mut state_guard,
                                &self.instruction_decoder,
                                &component.config,
                                &self.memory_translation_table,
                            );

                            time_slice -= 1;
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
                                            state_guard.address_bus as usize,
                                            component.config.assigned_address_space,
                                        )
                                        .unwrap_or_default();

                                    latch.push(byte);
                                    state_guard.address_bus =
                                        state_guard.address_bus.wrapping_add(1);

                                    tick = true;
                                }
                                Some(LoadStep::LatchToBus) => {
                                    match latch.len() {
                                        0 => {
                                            unreachable!()
                                        }
                                        1 => {
                                            state_guard.address_bus = latch[0] as u16;
                                        }
                                        2 => {
                                            let latch = [latch[0], latch[1]];
                                            state_guard.address_bus = u16::from_le_bytes(latch);
                                        }
                                        _ => {
                                            unreachable!()
                                        }
                                    }

                                    latch.clear();
                                }
                                Some(LoadStep::Offset { offset }) => {
                                    state_guard.address_bus =
                                        state_guard.address_bus.wrapping_add(offset as u16);
                                }
                                _ => unreachable!(),
                            }

                            if queue.is_empty() {
                                state_guard.execution_mode =
                                    Some(ExecutionMode::Interpret { instruction });
                            } else {
                                state_guard.execution_mode = Some(ExecutionMode::PreInterpret {
                                    instruction,
                                    latch,
                                    queue,
                                });
                            }

                            if tick {
                                time_slice -= 1;
                            }
                        }
                        ExecutionMode::Jammed => {
                            time_slice -= 1;
                        }
                        ExecutionMode::Wait => {
                            time_slice -= 1;
                        }
                        ExecutionMode::Reset => {
                            let program = [
                                self.memory_translation_table
                                    .read_le_value(
                                        RESET_VECTOR,
                                        component.config.assigned_address_space,
                                    )
                                    .unwrap_or_default(),
                                self.memory_translation_table
                                    .read_le_value(
                                        RESET_VECTOR + 1,
                                        component.config.assigned_address_space,
                                    )
                                    .unwrap_or_default(),
                            ];

                            state_guard.program = u16::from_le_bytes(program);
                            state_guard.execution_mode = Some(ExecutionMode::FetchAndDecode);

                            time_slice -= 1;
                        }
                        ExecutionMode::PostInterpret { mut queue } => {
                            match queue.pop_front() {
                                Some(PostInterpretStep::BusToProgram) => {
                                    state_guard.program = state_guard.address_bus;
                                }
                                Some(PostInterpretStep::Data { value }) => {
                                    let _ = self.memory_translation_table.write_le_value(
                                        state_guard.address_bus as usize,
                                        component.config.assigned_address_space,
                                        value,
                                    );
                                    state_guard.address_bus =
                                        state_guard.address_bus.wrapping_add(1);
                                }
                                Some(PostInterpretStep::PushStack { data }) => {
                                    state_guard.stack = state_guard.stack.wrapping_sub(1);
                                    let _ = self.memory_translation_table.write_le_value(
                                        STACK_BASE_ADDRESS + state_guard.stack as usize,
                                        component.config.assigned_address_space,
                                        data,
                                    );
                                }
                                Some(PostInterpretStep::AddToProgram { value }) => {
                                    state_guard.program =
                                        state_guard.program.wrapping_add_signed(value as i16);
                                }
                                _ => {
                                    unreachable!()
                                }
                            }

                            if queue.is_empty() {
                                state_guard.execution_mode = Some(ExecutionMode::FetchAndDecode);
                            } else {
                                state_guard.execution_mode =
                                    Some(ExecutionMode::PostInterpret { queue });
                            }

                            time_slice -= 1;
                        }
                        ExecutionMode::Interpret { instruction } => {
                            state_guard.execution_mode = Some(ExecutionMode::FetchAndDecode);

                            tracing::debug!("Interpreting instruction {:?}", instruction.opcode);

                            self.interpret_instruction(
                                &mut state_guard,
                                &component.config,
                                instruction.clone(),
                            );

                            time_slice -= 1;
                        }
                    }
                }
            })
            .unwrap();
    }
}

#[inline]
fn fetch_and_decode(
    state_guard: &mut ProcessorState,
    instruction_decoder: &Mos6502InstructionDecoder,
    config: &Mos6502Config,
    memory_translation_table: &MemoryAccessTable,
) {
    let (instruction, identifiying_bytes_length) = instruction_decoder
        .decode(
            state_guard.program as usize,
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

    state_guard.address_bus = state_guard
        .program
        .wrapping_add(identifiying_bytes_length as u16);
    state_guard.program = state_guard.program.wrapping_add(
        identifiying_bytes_length as u16
            + instruction
                .addressing_mode
                .map(|mode| mode.added_instruction_length())
                .unwrap_or(0),
    );

    let latch = ArrayVec::new();

    state_guard.execution_mode = Some(match instruction.addressing_mode {
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::Absolute)) => {
            ExecutionMode::PreInterpret {
                instruction,
                latch,
                queue: VecDeque::from_iter([LoadStep::Data, LoadStep::Data, LoadStep::LatchToBus]),
            }
        }
        Some(AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute)) => {
            let offset = state_guard.x;

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
            let offset = state_guard.y;

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
            let offset = state_guard.x;

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
            let offset = state_guard.y;

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
            let offset = state_guard.x;

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
            let offset = state_guard.y;

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
