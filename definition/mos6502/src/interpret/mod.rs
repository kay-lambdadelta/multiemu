use super::{
    FlagRegister, ProcessorState,
    instruction::{Mos6502InstructionSet, Mos6502Opcode},
};
use crate::{
    ExecutionStep, Mos6502Config,
    instruction::{AddressingMode, Mos6502AddressingMode, Opcode, Wdc65C02Opcode},
    task::CpuDriver,
};
use bitvec::{prelude::Msb0, view::BitView};
use multiemu_runtime::memory::Address;
use num::traits::{FromBytes, ToBytes};

pub const STACK_BASE_ADDRESS: Address = 0x0100;
const INTERRUPT_VECTOR: Address = 0xfffe;

// NOTE: https://www.pagetable.com/c64ref/6502

impl CpuDriver {
    pub(super) fn interpret_instruction(
        &self,
        state: &mut ProcessorState,
        config: &Mos6502Config,
        instruction: Mos6502InstructionSet,
    ) {
        match instruction.opcode {
            Opcode::Mos6502(opcode) => {
                match opcode {
                    Mos6502Opcode::Adc => {
                        let value: u8 = self.load(state, &instruction, config);

                        adc(state, config, value);
                    }
                    Mos6502Opcode::Anc => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = state.a & value;

                        state.flags.carry = result.view_bits::<Msb0>()[0];
                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.a = result;
                    }
                    Mos6502Opcode::And => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = state.a & value;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.a = result;
                    }
                    Mos6502Opcode::Arr => {
                        let value: u8 = self.load(state, &instruction, config);

                        let mut result = state.a & value;

                        let carry = state.flags.carry;
                        state.flags.carry = result.view_bits::<Msb0>()[0];

                        result >>= 1;

                        let result_bits = result.view_bits_mut::<Msb0>();
                        result_bits.set(0, carry);

                        state.flags.overflow = result_bits[1] != result_bits[0];
                        state.flags.negative = result_bits[0];
                        state.flags.zero = result == 0;

                        state.a = result;
                    }
                    Mos6502Opcode::Asl => {
                        let mut value: u8 = self.load(state, &instruction, config);
                        let value_bits = value.view_bits::<Msb0>();

                        let carry = value_bits[0];
                        let negative = value_bits[1];
                        value <<= 1;

                        state.flags.carry = carry;
                        state.flags.negative = negative;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            state.a = value;
                        } else {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Asr => {
                        let mut value: u8 = self.load(state, &instruction, config);
                        let value_bits = value.view_bits::<Msb0>();

                        let carry = value_bits[0];
                        let negative = value_bits[1];

                        value >>= 1;

                        state.flags.carry = carry;
                        state.flags.negative = negative;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            state.a = value;
                        } else {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Bcc => {
                        let value: i8 = self.load(state, &instruction, config);

                        if !state.flags.carry {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bcs => {
                        let value: i8 = self.load(state, &instruction, config);

                        if state.flags.carry {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Beq => {
                        let value: i8 = self.load(state, &instruction, config);

                        if state.flags.zero {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bit => {
                        let value: u8 = self.load(state, &instruction, config);
                        let value_bits = value.view_bits::<Msb0>();

                        let result = state.a & value;

                        state.flags.negative = value_bits[7];
                        state.flags.overflow = value_bits[6];
                        state.flags.zero = result == 0;
                    }
                    Mos6502Opcode::Bmi => {
                        let value: i8 = self.load(state, &instruction, config);

                        if state.flags.negative {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bne => {
                        let value: i8 = self.load(state, &instruction, config);

                        if !state.flags.zero {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bpl => {
                        let value: i8 = self.load(state, &instruction, config);

                        if !state.flags.negative {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Brk => {
                        let new_stack = state.stack.wrapping_sub(2);
                        let program_bytes = state.program.to_le_bytes();

                        let _ = self.memory_access_table.write_le_value(
                            new_stack as usize + STACK_BASE_ADDRESS,
                            config.assigned_address_space,
                            program_bytes[0],
                        );

                        let _ = self.memory_access_table.write_le_value(
                            new_stack as usize + STACK_BASE_ADDRESS + 1,
                            config.assigned_address_space,
                            program_bytes[1],
                        );

                        // https://www.nesdev.org/wiki/Status_flags
                        let mut flags = state.flags;
                        flags.undocumented = true;
                        flags.break_ = true;

                        let new_stack = new_stack.wrapping_sub(1);

                        let _ = self.memory_access_table.write_le_value(
                            new_stack as usize + STACK_BASE_ADDRESS,
                            config.assigned_address_space,
                            flags.to_byte(),
                        );

                        let program = [
                            self.memory_access_table
                                .read_le_value(INTERRUPT_VECTOR, config.assigned_address_space)
                                .unwrap_or_default(),
                            self.memory_access_table
                                .read_le_value(INTERRUPT_VECTOR + 1, config.assigned_address_space)
                                .unwrap_or_default(),
                        ];
                        state.program = u16::from_le_bytes(program);

                        state.stack = new_stack;
                    }
                    Mos6502Opcode::Bvc => {
                        let value: i8 = self.load(state, &instruction, config);

                        if !state.flags.overflow {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bvs => {
                        let value: i8 = self.load(state, &instruction, config);

                        if state.flags.overflow {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Clc => {
                        state.flags.carry = false;
                    }
                    Mos6502Opcode::Cld => {
                        state.flags.decimal = false;
                    }
                    Mos6502Opcode::Cli => {
                        state.flags.interrupt_disable = false;
                    }
                    Mos6502Opcode::Clv => {
                        state.flags.overflow = false;
                    }
                    Mos6502Opcode::Cmp => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = state.a.wrapping_sub(value);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;
                        state.flags.carry = state.a >= value;
                    }
                    Mos6502Opcode::Cpx => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = state.x.wrapping_sub(value);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;
                        state.flags.carry = state.x >= value;
                    }
                    Mos6502Opcode::Cpy => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = state.y.wrapping_sub(value);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;
                        state.flags.carry = state.x >= value;
                    }
                    Mos6502Opcode::Dcp => todo!(),
                    Mos6502Opcode::Dec => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = value.wrapping_sub(1);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            state.a = result;
                        } else {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(result));
                        }
                    }
                    Mos6502Opcode::Dex => {
                        let result = state.x.wrapping_sub(1);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.x = result;
                    }
                    Mos6502Opcode::Dey => {
                        let result = state.y.wrapping_sub(1);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.y = result;
                    }
                    Mos6502Opcode::Eor => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = state.a ^ value;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.a = result;
                    }
                    Mos6502Opcode::Inc => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = value.wrapping_add(1);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state
                            .execution_queue
                            .push_back(ExecutionStep::StoreData(result));
                    }
                    Mos6502Opcode::Inx => {
                        let result = state.x.wrapping_add(1);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.x = result;
                    }
                    Mos6502Opcode::Iny => {
                        let result = state.y.wrapping_add(1);

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.y = result;
                    }
                    Mos6502Opcode::Isc => todo!(),
                    Mos6502Opcode::Jam => {
                        tracing::error!(
                            "The MOS 6502 processor inside this machine just jammed itself"
                        );

                        state.execution_queue.push_back(ExecutionStep::Jammed);
                    }
                    Mos6502Opcode::Jmp => {
                        let value = match instruction.addressing_mode {
                            Some(AddressingMode::Mos6502(Mos6502AddressingMode::Absolute))
                            | Some(AddressingMode::Mos6502(
                                Mos6502AddressingMode::AbsoluteIndirect,
                            )) => state.address_bus,
                            _ => unreachable!(),
                        };

                        state.program = value;
                    }
                    Mos6502Opcode::Jsr => {
                        // We load the byte BEFORE the program counter
                        let program_bytes = (state.program.wrapping_sub(1)).to_le_bytes();

                        state.execution_queue.extend([
                            ExecutionStep::PushStack(program_bytes[1]),
                            ExecutionStep::PushStack(program_bytes[0]),
                            ExecutionStep::AddressBusToProgramPointer,
                        ]);
                    }
                    Mos6502Opcode::Las => todo!(),
                    Mos6502Opcode::Lax => {
                        let value: u8 = self.load(state, &instruction, config);

                        state.a = value;
                        state.x = value;
                    }
                    Mos6502Opcode::Lda => {
                        let value: u8 = self.load(state, &instruction, config);

                        state.flags.negative = value.view_bits::<Msb0>()[0];
                        state.flags.zero = value == 0;

                        state.a = value;
                    }
                    Mos6502Opcode::Ldx => {
                        let value: u8 = self.load(state, &instruction, config);

                        state.flags.negative = value.view_bits::<Msb0>()[0];
                        state.flags.zero = value == 0;

                        state.x = value;
                    }
                    Mos6502Opcode::Ldy => {
                        let value: u8 = self.load(state, &instruction, config);

                        state.flags.negative = value.view_bits::<Msb0>()[0];
                        state.flags.zero = value == 0;

                        state.y = value;
                    }
                    Mos6502Opcode::Lsr => {
                        let value: u8 = self.load(state, &instruction, config);
                        let value_bits = value.view_bits::<Msb0>();

                        let carry = value_bits[0];
                        let value = value >> 1;

                        state.flags.negative = false;
                        state.flags.carry = carry;
                        state.flags.zero = value == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            state.a = value;
                        } else {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Nop => {
                        if instruction.addressing_mode.is_some() {
                            let _: u8 = self.load(state, &instruction, config);
                        }
                    }
                    Mos6502Opcode::Ora => {
                        let value: u8 = self.load(state, &instruction, config);

                        let result = state.a | value;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.a = result;
                    }
                    Mos6502Opcode::Pha => {
                        state
                            .execution_queue
                            .push_back(ExecutionStep::PushStack(state.a));
                    }
                    Mos6502Opcode::Php => {
                        let mut flags = state.flags;
                        // https://www.nesdev.org/wiki/Status_flags
                        flags.undocumented = true;
                        flags.break_ = true;

                        state
                            .execution_queue
                            .push_back(ExecutionStep::PushStack(flags.to_byte()));
                    }
                    Mos6502Opcode::Pla => {
                        state.a = self
                            .memory_access_table
                            .read_le_value(
                                state.stack as usize + STACK_BASE_ADDRESS,
                                config.assigned_address_space,
                            )
                            .unwrap_or_default();

                        state.flags.negative = state.a.view_bits::<Msb0>()[0];
                        state.flags.zero = state.a == 0;

                        state.stack = state.stack.wrapping_add(1);
                    }
                    Mos6502Opcode::Plp => {
                        let value = self
                            .memory_access_table
                            .read_le_value(
                                state.stack as usize + STACK_BASE_ADDRESS,
                                config.assigned_address_space,
                            )
                            .unwrap_or_default();

                        state.flags = FlagRegister::from_byte(value);
                        state.stack = state.stack.wrapping_add(1);
                    }
                    Mos6502Opcode::Rla => todo!(),
                    Mos6502Opcode::Rol => {
                        let value: u8 = self.load(state, &instruction, config);
                        let value_bits = value.view_bits::<Msb0>();

                        state.flags.carry = value_bits[7];
                        state.flags.negative = value_bits[6];
                        let value = value.rotate_left(1);

                        state.flags.zero = value == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            state.a = value;
                        } else {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Ror => {
                        let value: u8 = self.load(state, &instruction, config);
                        let value_bits = value.view_bits::<Msb0>();

                        state.flags.carry = value_bits[0];
                        state.flags.negative = value_bits[1];
                        let value = value.rotate_right(1);
                        state.flags.zero = value == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            state.a = value;
                        } else {
                            state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Rra => todo!(),
                    Mos6502Opcode::Rti => {
                        let flags = self
                            .memory_access_table
                            .read_le_value::<u8>(
                                STACK_BASE_ADDRESS + state.stack as usize,
                                config.assigned_address_space,
                            )
                            .unwrap_or_default();

                        state.stack = state.stack.wrapping_add(1);
                        state.flags = FlagRegister::from_byte(flags);

                        let program = self
                            .memory_access_table
                            .read_le_value::<u16>(
                                STACK_BASE_ADDRESS + state.stack as usize,
                                config.assigned_address_space,
                            )
                            .unwrap_or_default();

                        state.stack = state.stack.wrapping_add(2);
                        state.program = program;
                    }
                    Mos6502Opcode::Rts => {
                        let program = self
                            .memory_access_table
                            .read_le_value::<u16>(
                                STACK_BASE_ADDRESS + state.stack as usize,
                                config.assigned_address_space,
                            )
                            .unwrap_or_default();

                        state.stack = state.stack.wrapping_add(2);
                        state.program = program.wrapping_add(1);
                    }
                    Mos6502Opcode::Sax => {
                        let value = state.a & state.x;

                        self.store(state, &instruction, config, value);
                    }
                    Mos6502Opcode::Sbc => {
                        let value: u8 = self.load(state, &instruction, config);

                        adc(state, config, !value);
                    }
                    Mos6502Opcode::Sbx => todo!(),
                    Mos6502Opcode::Sec => {
                        state.flags.carry = true;
                    }
                    Mos6502Opcode::Sed => {
                        state.flags.decimal = true;
                    }
                    Mos6502Opcode::Sei => {
                        state.flags.interrupt_disable = true;
                    }
                    Mos6502Opcode::Sha => todo!(),
                    Mos6502Opcode::Shs => todo!(),
                    Mos6502Opcode::Shx => todo!(),
                    Mos6502Opcode::Shy => todo!(),
                    Mos6502Opcode::Slo => todo!(),
                    Mos6502Opcode::Sre => todo!(),
                    Mos6502Opcode::Sta => {
                        self.store(state, &instruction, config, state.a);
                    }
                    Mos6502Opcode::Stx => {
                        self.store(state, &instruction, config, state.x);
                    }
                    Mos6502Opcode::Sty => {
                        self.store(state, &instruction, config, state.y);
                    }
                    Mos6502Opcode::Tax => {
                        let result = state.a;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.x = result;
                    }
                    Mos6502Opcode::Tay => {
                        let result = state.a;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.y = result;
                    }
                    Mos6502Opcode::Tsx => {
                        let result = state.stack;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.x = result;
                    }
                    Mos6502Opcode::Txa => {
                        let result = state.x;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.a = result;
                    }
                    Mos6502Opcode::Txs => {
                        state.stack = state.x;
                    }
                    Mos6502Opcode::Tya => {
                        let result = state.y;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = result == 0;

                        state.a = result;
                    }
                    Mos6502Opcode::Xaa => {
                        let value: u8 = self.load(state, &instruction, config);
                        let random_value: u8 = rand::random();

                        let result = (state.a & random_value) & state.x & value;

                        state.flags.negative = result.view_bits::<Msb0>()[0];
                        state.flags.zero = value == 0;

                        state.a = result;
                    }
                }
            }
            Opcode::Wdc65C02(opcode) => match opcode {
                Wdc65C02Opcode::Bra => {
                    let value: i8 = self.load(state, &instruction, config);

                    state
                        .execution_queue
                        .push_back(ExecutionStep::ModifyProgramPointer(value));
                }
                Wdc65C02Opcode::Phx => {
                    state
                        .execution_queue
                        .push_back(ExecutionStep::PushStack(state.x));
                }
                Wdc65C02Opcode::Phy => {
                    state
                        .execution_queue
                        .push_back(ExecutionStep::PushStack(state.y));
                }
                Wdc65C02Opcode::Plx => todo!(),
                Wdc65C02Opcode::Ply => todo!(),
                Wdc65C02Opcode::Stz => todo!(),
                Wdc65C02Opcode::Trb => todo!(),
                Wdc65C02Opcode::Tsb => todo!(),
                Wdc65C02Opcode::Stp => todo!(),
                Wdc65C02Opcode::Wai => todo!(),
            },
        }
    }

    #[inline]
    fn load<T: FromBytes<Bytes = [u8; 1]> + Default>(
        &self,
        state: &mut ProcessorState,
        instruction: &Mos6502InstructionSet,
        config: &Mos6502Config,
    ) -> T {
        match instruction.addressing_mode {
            Some(AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)) => {
                T::from_ne_bytes(&[state.a])
            }
            None => unreachable!(),
            _ => self
                .memory_access_table
                .read_le_value(state.address_bus as usize, config.assigned_address_space)
                .unwrap_or_default(),
        }
    }

    #[inline]
    fn store<T: ToBytes<Bytes = [u8; 1]>>(
        &self,
        state: &mut ProcessorState,
        instruction: &Mos6502InstructionSet,
        config: &Mos6502Config,
        value: T,
    ) {
        match instruction.addressing_mode {
            Some(AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)) => {
                state.a = value.to_ne_bytes()[0];
            }
            None => {
                unreachable!()
            }
            _ => {
                let _ = self.memory_access_table.write_le_value(
                    state.address_bus as usize,
                    config.assigned_address_space,
                    value,
                );
            }
        }
    }
}

#[inline]
fn adc(state: &mut ProcessorState, config: &Mos6502Config, value: u8) {
    if state.flags.decimal && config.kind.supports_decimal() {
    } else {
        let carry = state.flags.carry as u8;

        let (first_operation_result, first_operation_carry) = state.a.overflowing_add(value);

        let (second_operation_result, second_operation_carry) =
            first_operation_result.overflowing_add(carry);

        let a_bits = state.a.view_bits::<Msb0>();
        let value_bits = value.view_bits::<Msb0>();
        let result_bits = second_operation_result.view_bits::<Msb0>();

        state.flags.overflow = (a_bits[0] == value_bits[0]) && (a_bits[0] != result_bits[0]);
        state.flags.carry = first_operation_carry || second_operation_carry;
        state.flags.negative = result_bits[0];
        state.flags.zero = second_operation_result == 0;

        state.a = second_operation_result;
    }
}
