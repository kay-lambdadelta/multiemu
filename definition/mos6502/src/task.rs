use crate::{
    ExecutionMode, LoadStep, Mos6502, PostInterpretStep, RESET_VECTOR,
    decoder::Mos6502InstructionDecoder,
    instruction::{AddressingMode, Mos6502AddressingMode, Wdc65C02AddressingMode},
    interpret::STACK_BASE_ADDRESS,
};
use arrayvec::ArrayVec;
use multiemu_runtime::{
    component::ComponentRef, memory::MemoryAccessTable, processor::InstructionDecoder,
    scheduler::Task,
};
use std::{collections::VecDeque, num::NonZero, sync::Arc};

pub struct CpuDriver {
    pub memory_access_table: Arc<MemoryAccessTable>,
    pub instruction_decoder: Mos6502InstructionDecoder,
    pub component: ComponentRef<Mos6502>,
}

impl Task for CpuDriver {
    fn run(&mut self, time_slice: NonZero<u32>) {
        self.component
            .interact_mut(|component| {
                // Keep the guard like this so doing a full load of the guard only happens occasionally
                let mut time_slice = time_slice.get();

                while time_slice != 0 {
                    if !component.rdy.load() {
                        time_slice -= 1;
                        continue;
                    }

                    match component.state.execution_mode.take().unwrap() {
                        ExecutionMode::FetchAndDecode => {
                            tracing::debug!(
                                "Fetching and decoding instruction from {:#04x}",
                                component.state.program
                            );

                            component.fetch_and_decode(
                                &self.instruction_decoder,
                                &self.memory_access_table,
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
                                        .memory_access_table
                                        .read_le_value(
                                            component.state.address_bus as usize,
                                            component.config.assigned_address_space,
                                        )
                                        .unwrap_or_default();

                                    latch.push(byte);
                                    component.state.address_bus =
                                        component.state.address_bus.wrapping_add(1);

                                    tick = true;
                                }
                                Some(LoadStep::LatchToBus) => {
                                    match latch.len() {
                                        0 => {
                                            unreachable!()
                                        }
                                        1 => {
                                            component.state.address_bus = latch[0] as u16;
                                        }
                                        2 => {
                                            let latch = [latch[0], latch[1]];
                                            component.state.address_bus = u16::from_le_bytes(latch);
                                        }
                                        _ => {
                                            unreachable!()
                                        }
                                    }

                                    latch.clear();
                                }
                                Some(LoadStep::Offset { offset }) => {
                                    component.state.address_bus =
                                        component.state.address_bus.wrapping_add(offset as u16);
                                }
                                _ => unreachable!(),
                            }

                            if queue.is_empty() {
                                component.state.execution_mode =
                                    Some(ExecutionMode::Interpret { instruction });
                            } else {
                                component.state.execution_mode =
                                    Some(ExecutionMode::PreInterpret {
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
                                self.memory_access_table
                                    .read_le_value(
                                        RESET_VECTOR,
                                        component.config.assigned_address_space,
                                    )
                                    .unwrap_or_default(),
                                self.memory_access_table
                                    .read_le_value(
                                        RESET_VECTOR + 1,
                                        component.config.assigned_address_space,
                                    )
                                    .unwrap_or_default(),
                            ];

                            component.state.program = u16::from_le_bytes(program);
                            component.state.execution_mode = Some(ExecutionMode::FetchAndDecode);

                            time_slice -= 1;
                        }
                        ExecutionMode::PostInterpret { mut queue } => {
                            match queue.pop_front() {
                                Some(PostInterpretStep::BusToProgram) => {
                                    component.state.program = component.state.address_bus;
                                }
                                Some(PostInterpretStep::Data { value }) => {
                                    let _ = self.memory_access_table.write_le_value(
                                        component.state.address_bus as usize,
                                        component.config.assigned_address_space,
                                        value,
                                    );
                                    component.state.address_bus =
                                        component.state.address_bus.wrapping_add(1);
                                }
                                Some(PostInterpretStep::PushStack { data }) => {
                                    component.state.stack = component.state.stack.wrapping_sub(1);
                                    let _ = self.memory_access_table.write_le_value(
                                        STACK_BASE_ADDRESS + component.state.stack as usize,
                                        component.config.assigned_address_space,
                                        data,
                                    );
                                }
                                Some(PostInterpretStep::AddToProgram { value }) => {
                                    component.state.program =
                                        component.state.program.wrapping_add_signed(value as i16);
                                }
                                _ => {
                                    unreachable!()
                                }
                            }

                            if queue.is_empty() {
                                component.state.execution_mode =
                                    Some(ExecutionMode::FetchAndDecode);
                            } else {
                                component.state.execution_mode =
                                    Some(ExecutionMode::PostInterpret { queue });
                            }

                            time_slice -= 1;
                        }
                        ExecutionMode::Interpret { instruction } => {
                            component.state.execution_mode = Some(ExecutionMode::FetchAndDecode);

                            tracing::debug!("Interpreting instruction {:?}", instruction.opcode);

                            self.interpret_instruction(
                                &mut component.state,
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

impl Mos6502 {
    #[inline]
    fn fetch_and_decode(
        &mut self,
        instruction_decoder: &Mos6502InstructionDecoder,
        memory_access_table: &MemoryAccessTable,
    ) {
        let (instruction, identifiying_bytes_length) = instruction_decoder
            .decode(
                self.state.program as usize,
                self.config.assigned_address_space,
                memory_access_table,
            )
            .unwrap();

        debug_assert!(
            instruction
                .addressing_mode
                .map(|addressing_mode| { addressing_mode.is_valid_for_mode(self.config.kind) })
                .unwrap_or(true),
            "Invalid addressing mode for instruction for mode {:?}: {:?}",
            self.config.kind,
            instruction,
        );

        tracing::debug!("Decoded instruction: {:#?}", instruction);

        self.state.address_bus = self
            .state
            .program
            .wrapping_add(identifiying_bytes_length as u16);
        self.state.program = self.state.program.wrapping_add(
            identifiying_bytes_length as u16
                + instruction
                    .addressing_mode
                    .map(|mode| mode.added_instruction_length())
                    .unwrap_or(0),
        );

        let latch = ArrayVec::new();

        self.state.execution_mode = Some(match instruction.addressing_mode {
            Some(AddressingMode::Mos6502(Mos6502AddressingMode::Absolute)) => {
                ExecutionMode::PreInterpret {
                    instruction,
                    latch,
                    queue: VecDeque::from_iter([
                        LoadStep::Data,
                        LoadStep::Data,
                        LoadStep::LatchToBus,
                    ]),
                }
            }
            Some(AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute)) => {
                let offset = self.state.x;

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
                let offset = self.state.y;

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
                let offset = self.state.x;

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
                let offset = self.state.y;

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
                let offset = self.state.x;

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
                let offset = self.state.y;

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
}
