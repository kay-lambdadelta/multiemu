use super::{
    FlagRegister, ProcessorState,
    instruction::{Mos6502InstructionSet, Opcode},
};
use crate::{ExecutionMode, StoreStep, instruction::AddressingMode, task::Mos6502Task};
use bitvec::{prelude::Msb0, view::BitView};
use enumflags2::BitFlag;
use multiemu_machine::memory::memory_translation_table::MemoryTranslationTable;
use num::traits::{FromBytes, ToBytes};
use std::collections::VecDeque;

pub const STACK_BASE_ADDRESS: usize = 0x0100;
const INTERRUPT_VECTOR: usize = 0xfffe;

#[cfg(test)]
mod tests;

// NOTE: https://www.pagetable.com/c64ref/6502

impl Mos6502Task {
    pub(super) fn interpret_instruction(
        &self,
        state: &mut ProcessorState,
        instruction: Mos6502InstructionSet,
    ) {
        let memory_translation_table = &self.essentials.memory_translation_table;

        match instruction.opcode {
            Opcode::Adc => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                if state.flags.contains(FlagRegister::Decimal)
                    && self.config.kind.supports_decimal()
                {
                } else {
                    let carry = state.flags.contains(FlagRegister::Carry) as u8;

                    let (first_operation_result, first_operation_overflow) =
                        state.a.overflowing_add(value);

                    let (second_operation_result, second_operation_overflow) =
                        first_operation_result.overflowing_add(carry);

                    state.flags.set(
                        FlagRegister::Overflow,
                        // If it overflowed at any point this is set
                        first_operation_overflow || second_operation_overflow,
                    );

                    state.flags.set(
                        FlagRegister::Carry,
                        first_operation_overflow || second_operation_overflow,
                    );

                    state.flags.set(
                        FlagRegister::Negative,
                        second_operation_result.view_bits::<Msb0>()[0],
                    );
                    state
                        .flags
                        .set(FlagRegister::Zero, second_operation_result == 0);

                    state.a = second_operation_result;
                }
            }
            Opcode::Anc => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = state.a & value;

                state.flags.set(
                    FlagRegister::Carry | FlagRegister::Negative,
                    result.view_bits::<Msb0>()[0],
                );
                state.flags.set(FlagRegister::Zero, result == 0);

                state.a = result;
            }
            Opcode::And => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = state.a & value;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.a = result;
            }
            Opcode::Arr => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let mut result = state.a & value;

                let carry = state.flags.contains(FlagRegister::Carry);
                state
                    .flags
                    .set(FlagRegister::Carry, result.view_bits::<Msb0>()[0]);

                result >>= 1;

                let result_bits = result.view_bits_mut::<Msb0>();
                result_bits.set(0, carry);

                state
                    .flags
                    .set(FlagRegister::Overflow, result_bits[1] != result_bits[0]);
                state.flags.set(FlagRegister::Negative, result_bits[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.a = result;
            }
            Opcode::Asl => {
                let mut value: u8 = self.load(state, &instruction, memory_translation_table);
                let value_bits = value.view_bits::<Msb0>();

                let carry = value_bits[0];
                let negative = value_bits[1];
                value <<= 1;

                state.flags.set(FlagRegister::Carry, carry);
                state.flags.set(FlagRegister::Negative, negative);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.a = value;
                } else {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::Data { value }]),
                    });
                }
            }
            Opcode::Asr => {
                let mut value: u8 = self.load(state, &instruction, memory_translation_table);
                let value_bits = value.view_bits::<Msb0>();

                let carry = value_bits[0];
                let negative = value_bits[1];

                value >>= 1;

                state.flags.set(FlagRegister::Carry, carry);
                state.flags.set(FlagRegister::Negative, negative);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.a = value;
                } else {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::Data { value }]),
                    });
                }
            }
            Opcode::Bcc => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if !state.flags.contains(FlagRegister::Carry) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Bcs => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if state.flags.contains(FlagRegister::Carry) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Beq => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if state.flags.contains(FlagRegister::Zero) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Bit => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);
                let value_bits = value.view_bits::<Msb0>();

                let result = state.a & value;

                state.flags.set(FlagRegister::Negative, value_bits[7]);
                state.flags.set(FlagRegister::Overflow, value_bits[6]);
                state.flags.set(FlagRegister::Zero, result == 0);
            }
            Opcode::Bmi => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if state.flags.contains(FlagRegister::Negative) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Bne => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if !state.flags.contains(FlagRegister::Zero) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Bpl => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if !state.flags.contains(FlagRegister::Negative) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Brk => {
                let new_stack = state.stack.wrapping_sub(2);
                let program_bytes = state.program.to_le_bytes();

                let _ = memory_translation_table.write_le_value(
                    new_stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    program_bytes[0],
                );

                let _ = memory_translation_table.write_le_value(
                    new_stack as usize + STACK_BASE_ADDRESS + 1,
                    self.config.assigned_address_space,
                    program_bytes[1],
                );

                // https://www.nesdev.org/wiki/Status_flags
                let mut flags = state.flags;
                flags.insert(FlagRegister::__Unused);
                flags.insert(FlagRegister::Break);

                let new_stack = new_stack.wrapping_sub(1);

                let _ = memory_translation_table.write_le_value(
                    new_stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    flags.bits(),
                );

                let program = [
                    memory_translation_table
                        .read_le_value(INTERRUPT_VECTOR, self.config.assigned_address_space)
                        .unwrap_or_default(),
                    memory_translation_table
                        .read_le_value(INTERRUPT_VECTOR + 1, self.config.assigned_address_space)
                        .unwrap_or_default(),
                ];
                state.program = u16::from_le_bytes(program);

                state.stack = new_stack;
            }
            Opcode::Bvc => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if !state.flags.contains(FlagRegister::Overflow) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Bvs => {
                let value: i8 = self.load(state, &instruction, memory_translation_table);

                if state.flags.contains(FlagRegister::Overflow) {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::AddToProgram { value }]),
                    });
                }
            }
            Opcode::Clc => {
                state.flags.remove(FlagRegister::Carry);
            }
            Opcode::Cld => {
                state.flags.remove(FlagRegister::Decimal);
            }
            Opcode::Cli => {
                state.flags.remove(FlagRegister::InterruptDisable);
            }
            Opcode::Clv => {
                state.flags.remove(FlagRegister::Overflow);
            }
            Opcode::Cmp => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = state.a.wrapping_sub(value);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);
                state.flags.set(FlagRegister::Carry, state.a >= value);
            }
            Opcode::Cpx => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = state.x.wrapping_sub(value);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);
                state.flags.set(FlagRegister::Carry, state.x >= value);
            }
            Opcode::Cpy => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = state.y.wrapping_sub(value);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);
                state.flags.set(FlagRegister::Carry, state.x >= value);
            }
            Opcode::Dcp => todo!(),
            Opcode::Dec => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = value.wrapping_sub(1);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.a = result;
                } else {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::Data { value: result }]),
                    });
                }
            }
            Opcode::Dex => {
                let result = state.x.wrapping_sub(1);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.x = result;
            }
            Opcode::Dey => {
                let result = state.y.wrapping_sub(1);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.y = result;
            }
            Opcode::Eor => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = state.a ^ value;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.a = result;
            }
            Opcode::Inc => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = value.wrapping_add(1);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.execution_mode = Some(ExecutionMode::Store {
                    queue: VecDeque::from_iter([StoreStep::Data { value: result }]),
                });
            }
            Opcode::Inx => {
                let result = state.x.wrapping_add(1);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.x = result;
            }
            Opcode::Iny => {
                let result = state.y.wrapping_add(1);

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.y = result;
            }
            Opcode::Isc => todo!(),
            Opcode::Jam => {
                tracing::error!("The MOS 6502 processor inside this machine just jammed itself");

                state.execution_mode = Some(ExecutionMode::Jammed);
            }
            Opcode::Jmp => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Absolute) | Some(AddressingMode::AbsoluteIndirect) => {
                        state.address_bus
                    }
                    _ => unreachable!(),
                };

                state.program = value;
            }
            Opcode::Jsr => {
                let program_bytes = state.program.to_le_bytes();

                state.execution_mode = Some(ExecutionMode::Store {
                    queue: VecDeque::from_iter([
                        StoreStep::PushStack {
                            data: program_bytes[0],
                        },
                        StoreStep::PushStack {
                            data: program_bytes[1],
                        },
                        StoreStep::BusToProgram,
                    ]),
                })
            }
            Opcode::Las => todo!(),
            Opcode::Lax => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                state.a = value;
                state.x = value;
            }
            Opcode::Lda => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                state
                    .flags
                    .set(FlagRegister::Negative, value.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, value == 0);

                state.a = value;
            }
            Opcode::Ldx => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                state
                    .flags
                    .set(FlagRegister::Negative, value.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, value == 0);

                state.x = value;
            }
            Opcode::Ldy => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                state
                    .flags
                    .set(FlagRegister::Negative, value.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, value == 0);

                state.y = value;
            }
            Opcode::Lsr => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);
                let value_bits = value.view_bits::<Msb0>();

                let carry = value_bits[0];
                let value = value >> 1;

                state.flags.remove(FlagRegister::Negative);
                state.flags.set(FlagRegister::Carry, carry);
                state.flags.set(FlagRegister::Zero, value == 0);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.a = value;
                } else {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::Data { value }]),
                    });
                }
            }
            Opcode::Nop => {
                if instruction.addressing_mode.is_some() {
                    let _: u8 = self.load(state, &instruction, memory_translation_table);
                }
            }
            Opcode::Ora => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                let result = state.a | value;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.a = result;
            }
            Opcode::Pha => {
                state.execution_mode = Some(ExecutionMode::Store {
                    queue: VecDeque::from_iter([StoreStep::PushStack { data: state.a }]),
                });
            }
            Opcode::Php => {
                let mut flags = state.flags;
                // https://www.nesdev.org/wiki/Status_flags
                flags.insert(FlagRegister::__Unused);
                flags.insert(FlagRegister::Break);

                state.execution_mode = Some(ExecutionMode::Store {
                    queue: VecDeque::from_iter([StoreStep::PushStack { data: flags.bits() }]),
                });
            }
            Opcode::Pla => {
                state.a = memory_translation_table
                    .read_le_value(
                        state.stack as usize + STACK_BASE_ADDRESS,
                        self.config.assigned_address_space,
                    )
                    .unwrap_or_default();

                state
                    .flags
                    .set(FlagRegister::Negative, state.a.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, state.a == 0);

                state.stack = state.stack.wrapping_add(1);
            }
            Opcode::Plp => {
                let value = memory_translation_table
                    .read_le_value(
                        state.stack as usize + STACK_BASE_ADDRESS,
                        self.config.assigned_address_space,
                    )
                    .unwrap_or_default();

                state.flags = FlagRegister::from_bits(value).unwrap();
                state.stack = state.stack.wrapping_add(1);
            }
            Opcode::Rla => todo!(),
            Opcode::Rol => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);
                let value_bits = value.view_bits::<Msb0>();

                let carry = value_bits[7];
                let negative = value_bits[6];
                let value = value.rotate_left(1);

                state.flags.set(FlagRegister::Carry, carry);
                state.flags.set(FlagRegister::Negative, negative);
                state.flags.set(FlagRegister::Zero, value == 0);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.a = value;
                } else {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::Data { value }]),
                    });
                }
            }
            Opcode::Ror => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);
                let value_bits = value.view_bits::<Msb0>();

                let carry = value_bits[0];
                let negative = value_bits[1];
                let value = value.rotate_right(1);

                state.flags.set(FlagRegister::Carry, carry);
                state.flags.set(FlagRegister::Negative, negative);
                state.flags.set(FlagRegister::Zero, value == 0);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.a = value;
                } else {
                    state.execution_mode = Some(ExecutionMode::Store {
                        queue: VecDeque::from_iter([StoreStep::Data { value }]),
                    });
                }
            }
            Opcode::Rra => todo!(),
            Opcode::Rti => todo!(),
            Opcode::Rts => {
                let program = [
                    memory_translation_table
                        .read_le_value(
                            STACK_BASE_ADDRESS + state.stack as usize,
                            self.config.assigned_address_space,
                        )
                        .unwrap_or_default(),
                    memory_translation_table
                        .read_le_value(
                            STACK_BASE_ADDRESS + state.stack.wrapping_add(1) as usize,
                            self.config.assigned_address_space,
                        )
                        .unwrap_or_default(),
                ];

                state.stack = state.stack.wrapping_add(2);
                state.program = u16::from_le_bytes(program);
            }
            Opcode::Sax => {
                let value = state.a & state.x;

                self.store(state, &instruction, memory_translation_table, value);
            }
            Opcode::Sbc => {
                let value: u8 = self.load(state, &instruction, memory_translation_table);

                if state.flags.contains(FlagRegister::Decimal)
                    && self.config.kind.supports_decimal()
                {
                } else {
                    let carry = state.flags.contains(FlagRegister::Carry) as u8;

                    let (first_operation_result, first_operation_overflow) =
                        state.a.overflowing_sub(value);

                    let (second_operation_result, second_operation_overflow) =
                        first_operation_result.overflowing_sub(carry);

                    state.flags.set(
                        FlagRegister::Overflow,
                        // If it overflowed at any point this is set
                        first_operation_overflow || second_operation_overflow,
                    );

                    state.flags.set(
                        FlagRegister::Carry,
                        first_operation_overflow || second_operation_overflow,
                    );

                    state.flags.set(
                        FlagRegister::Negative,
                        second_operation_result.view_bits::<Msb0>()[0],
                    );
                    state
                        .flags
                        .set(FlagRegister::Zero, second_operation_result == 0);

                    state.a = second_operation_result;
                }
            }
            Opcode::Sbx => todo!(),
            Opcode::Sec => {
                state.flags.insert(FlagRegister::Carry);
            }
            Opcode::Sed => {
                state.flags.insert(FlagRegister::Decimal);
            }
            Opcode::Sei => {
                state.flags.insert(FlagRegister::InterruptDisable);
            }
            Opcode::Sha => todo!(),
            Opcode::Shs => todo!(),
            Opcode::Shx => todo!(),
            Opcode::Shy => todo!(),
            Opcode::Slo => todo!(),
            Opcode::Sre => todo!(),
            Opcode::Sta => {
                self.store(state, &instruction, memory_translation_table, state.a);
            }
            Opcode::Stx => {
                self.store(state, &instruction, memory_translation_table, state.x);
            }
            Opcode::Sty => {
                self.store(state, &instruction, memory_translation_table, state.y);
            }
            Opcode::Tax => {
                let result = state.a;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.x = result;
            }
            Opcode::Tay => {
                let result = state.a;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.y = result;
            }
            Opcode::Tsx => {
                let result = state.stack;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.x = result;
            }
            Opcode::Txa => {
                let result = state.x;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.a = result;
            }
            Opcode::Txs => {
                state.stack = state.x;
            }
            Opcode::Tya => {
                let result = state.y;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, result == 0);

                state.a = result;
            }
            Opcode::Xaa => {
                tracing::warn!("Program used XAA instruction which is highly unpredictable");

                let value: u8 = self.load(state, &instruction, memory_translation_table);
                let random_value: u8 = rand::random();

                let result = (state.a & random_value) & state.x & value;

                state
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.flags.set(FlagRegister::Zero, value == 0);

                state.a = result;
            }
        }
    }

    fn load<T: FromBytes<Bytes = [u8; 1]> + Default>(
        &self,
        state: &mut ProcessorState,
        instruction: &Mos6502InstructionSet,
        memory_translation_table: &MemoryTranslationTable,
    ) -> T {
        match instruction.addressing_mode {
            Some(AddressingMode::Accumulator) => T::from_ne_bytes(&[state.a]),
            Some(AddressingMode::Immediate)
            | Some(AddressingMode::XIndexedAbsolute)
            | Some(AddressingMode::YIndexedAbsolute)
            | Some(AddressingMode::AbsoluteIndirect)
            | Some(AddressingMode::ZeroPage)
            | Some(AddressingMode::XIndexedZeroPage)
            | Some(AddressingMode::YIndexedZeroPage)
            | Some(AddressingMode::XIndexedZeroPageIndirect)
            | Some(AddressingMode::ZeroPageIndirectYIndexed)
            | Some(AddressingMode::Relative)
            | Some(AddressingMode::Absolute) => memory_translation_table
                .read_le_value(
                    state.address_bus as usize,
                    self.config.assigned_address_space,
                )
                .unwrap_or_default(),
            None => todo!(),
        }
    }

    fn store<T: ToBytes<Bytes = [u8; 1]>>(
        &self,
        state: &mut ProcessorState,
        instruction: &Mos6502InstructionSet,
        memory_translation_table: &MemoryTranslationTable,
        value: T,
    ) {
        match instruction.addressing_mode {
            Some(AddressingMode::Accumulator) => {
                state.a = value.to_ne_bytes()[0];
            }
            Some(AddressingMode::Immediate)
            | Some(AddressingMode::XIndexedAbsolute)
            | Some(AddressingMode::YIndexedAbsolute)
            | Some(AddressingMode::AbsoluteIndirect)
            | Some(AddressingMode::ZeroPage)
            | Some(AddressingMode::XIndexedZeroPage)
            | Some(AddressingMode::YIndexedZeroPage)
            | Some(AddressingMode::XIndexedZeroPageIndirect)
            | Some(AddressingMode::ZeroPageIndirectYIndexed)
            | Some(AddressingMode::Relative)
            | Some(AddressingMode::Absolute) => {
                let _ = memory_translation_table.write_le_value(
                    state.address_bus as usize,
                    self.config.assigned_address_space,
                    value,
                );
            }
            None => {
                todo!()
            }
        }
    }
}
