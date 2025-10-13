use crate::{
    AddressBusModification, ExecutionStep, IRQ_VECTOR, Mos6502, NMI_VECTOR, ProcessorState,
    RESET_VECTOR,
    decoder::Mos6502InstructionDecoder,
    instruction::{AddressingMode, Mos6502AddressingMode, Wdc65C02AddressingMode},
    interpret::STACK_BASE_ADDRESS,
};
use multiemu_base::{memory::MemoryAccessTable, processor::InstructionDecoder, scheduler::TaskMut};
use std::{num::NonZero, sync::Arc};

#[derive(Debug)]
pub struct Driver {
    pub memory_access_table: Arc<MemoryAccessTable>,
    pub instruction_decoder: Mos6502InstructionDecoder,
}

impl TaskMut<Mos6502> for Driver {
    fn run(&mut self, component: &mut Mos6502, time_slice: NonZero<u32>) {
        let mut time_slice = time_slice.get();

        while time_slice != 0 {
            if !component.rdy.load() {
                time_slice -= 1;
                continue;
            }

            match component.state.execution_queue.pop_front().unwrap() {
                ExecutionStep::Reset => {
                    component.state.interrupt(RESET_VECTOR, false, false);

                    time_slice -= 1;
                }
                ExecutionStep::Jammed => {
                    time_slice -= 1;

                    component.state.execution_queue.clear();
                    component
                        .state
                        .execution_queue
                        .push_back(ExecutionStep::Jammed);
                }
                ExecutionStep::Wait => {
                    time_slice -= 1;

                    component
                        .state
                        .execution_queue
                        .push_back(ExecutionStep::Wait);
                }
                ExecutionStep::FetchAndDecode => {
                    if component.config.kind.supports_interrupts() {
                        if component.nmi.interrupt_required() {
                            component.state.interrupt(NMI_VECTOR, false, true);
                        } else if component.irq.interrupt_required()
                            && !component.state.flags.interrupt_disable
                        {
                            component.state.interrupt(IRQ_VECTOR, true, true);
                        } else {
                            component.fetch_and_decode(
                                &self.instruction_decoder,
                                &self.memory_access_table,
                            );
                        }
                    } else {
                        component
                            .fetch_and_decode(&self.instruction_decoder, &self.memory_access_table);
                    }

                    time_slice -= 1;
                }
                ExecutionStep::LoadData => {
                    let byte = self
                        .memory_access_table
                        .read_le_value(
                            component.state.address_bus as usize,
                            component.config.assigned_address_space,
                        )
                        .unwrap_or_default();

                    component.state.latch.push(byte);
                    component.state.address_bus = component.state.address_bus.wrapping_add(1);

                    time_slice -= 1;
                }
                ExecutionStep::LoadDataFromConstant(data) => {
                    component.state.latch.push(data);
                    time_slice -= 1;
                }
                ExecutionStep::StoreData(data) => {
                    let _ = self.memory_access_table.write_le_value(
                        component.state.address_bus as usize,
                        component.config.assigned_address_space,
                        data,
                    );
                    component.state.address_bus = component.state.address_bus.wrapping_add(1);

                    time_slice -= 1;
                }
                ExecutionStep::PushStack(data) => {
                    let _ = self.memory_access_table.write_le_value(
                        STACK_BASE_ADDRESS + component.state.stack as usize,
                        component.config.assigned_address_space,
                        data,
                    );
                    component.state.stack = component.state.stack.wrapping_sub(1);

                    time_slice -= 1;
                }
                ExecutionStep::LatchToAddressBus => {
                    match component.state.latch.len() {
                        1 => {
                            component.state.address_bus = component.state.latch[0] as u16;
                        }
                        2 => {
                            let latch = [component.state.latch[0], component.state.latch[1]];
                            component.state.address_bus = u16::from_le_bytes(latch);
                        }
                        _ => {
                            unreachable!()
                        }
                    }

                    component.state.latch.clear();
                }
                // Literally only used for interrupts
                ExecutionStep::LatchToProgramPointer => {
                    assert!(component.state.latch.len() == 2);

                    component.state.program =
                        u16::from_le_bytes([component.state.latch[0], component.state.latch[1]]);
                    component.state.latch.clear();
                }
                ExecutionStep::AddressBusToProgramPointer => {
                    component.state.program = component.state.address_bus;
                    time_slice -= 1;
                }
                ExecutionStep::ModifyProgramPointer(value) => {
                    component.state.program =
                        component.state.program.wrapping_add_signed(value as i16);
                    time_slice -= 1;
                }
                ExecutionStep::ModifyAddressBus {
                    modification,
                    zero_page,
                } => {
                    let modification = match modification {
                        AddressBusModification::X => component.state.x,
                        AddressBusModification::Y => component.state.y,
                    };

                    component.state.address_bus = component
                        .state
                        .address_bus
                        .wrapping_add(modification as u16);

                    if zero_page {
                        component.state.address_bus %= 0x100;
                    }
                }
                ExecutionStep::Interpret { instruction } => {
                    self.interpret_instruction(
                        &mut component.state,
                        &component.config,
                        instruction.clone(),
                    );

                    component
                        .state
                        .execution_queue
                        .push_back(ExecutionStep::FetchAndDecode);

                    time_slice -= 1;
                }
            }
        }
    }
}

impl Mos6502 {
    #[inline]
    fn fetch_and_decode(
        &mut self,
        instruction_decoder: &Mos6502InstructionDecoder,
        memory_access_table: &MemoryAccessTable,
    ) {
        let (instruction, identifying_bytes_length) = instruction_decoder
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

        self.state.address_bus = self
            .state
            .program
            .wrapping_add(identifying_bytes_length as u16);
        self.state.program = self.state.program.wrapping_add(
            identifying_bytes_length as u16
                + instruction
                    .addressing_mode
                    .map(|mode| mode.added_instruction_length())
                    .unwrap_or(0),
        );
        self.state.latch.clear();

        tracing::trace!("{:?} {:04x?}", instruction, self.state);

        if let Some(addressing_mode) = instruction.addressing_mode {
            match addressing_mode {
                AddressingMode::Mos6502(Mos6502AddressingMode::Absolute) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::Immediate) => {}
                AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus {
                            modification: AddressBusModification::X,
                            zero_page: false,
                        },
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus {
                            modification: AddressBusModification::Y,
                            zero_page: false,
                        },
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::AbsoluteIndirect) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPageIndirect) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus {
                            modification: AddressBusModification::X,
                            zero_page: true,
                        },
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPageIndirectYIndexed) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus {
                            modification: AddressBusModification::Y,
                            zero_page: false,
                        },
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus {
                            modification: AddressBusModification::X,
                            zero_page: true,
                        },
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedZeroPage) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus {
                            modification: AddressBusModification::Y,
                            zero_page: true,
                        },
                    ]);
                }

                AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPage) => {
                    self.state
                        .execution_queue
                        .extend([ExecutionStep::LoadData, ExecutionStep::LatchToAddressBus]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::Relative) => {}
                AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator) => {}
                AddressingMode::Wdc65C02(Wdc65C02AddressingMode::ZeroPageIndirect) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                    ]);
                }
            }
        }

        self.state
            .execution_queue
            .push_back(ExecutionStep::Interpret { instruction });
    }
}

impl ProcessorState {
    pub fn interrupt(&mut self, vector: u16, break_status: bool, save_current_state: bool) {
        let vector = vector.to_le_bytes();
        let mut flags = self.flags;

        flags.break_ = break_status;
        flags.undocumented = true;
        flags.interrupt_disable = true;

        if save_current_state {
            let program_pointer = self.program.to_le_bytes();

            self.execution_queue.extend([
                ExecutionStep::PushStack(program_pointer[1]),
                ExecutionStep::PushStack(program_pointer[0]),
                ExecutionStep::PushStack(flags.to_byte()),
            ]);
        }

        self.execution_queue.extend([
            ExecutionStep::LoadDataFromConstant(vector[0]),
            ExecutionStep::LoadDataFromConstant(vector[1]),
            ExecutionStep::LatchToAddressBus,
            ExecutionStep::LoadData,
            ExecutionStep::LoadData,
            ExecutionStep::LatchToProgramPointer,
            ExecutionStep::FetchAndDecode,
        ]);
    }
}
