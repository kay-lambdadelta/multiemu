use bitvec::{prelude::Lsb0, view::BitView};
use multiemu_runtime::memory::Address;

use super::{
    FlagRegister,
    instruction::{Mos6502InstructionSet, Mos6502Opcode},
};
use crate::{
    ExecutionStep, Mos6502,
    instruction::{AddressingMode, Mos6502AddressingMode, Opcode, Wdc65C02Opcode},
};

pub const STACK_BASE_ADDRESS: Address = 0x0100;
const INTERRUPT_VECTOR: Address = 0xfffe;

// NOTE: https://www.pagetable.com/c64ref/6502

// FIXME: Page crossing cycle penalties are not emulated
// FIXME: Many undocumented instructions are either incorrect or not emulated at
// all FIXME: Decimal mode is not implemented

impl Mos6502 {
    #[inline]
    pub(super) fn interpret_instruction(&mut self, instruction: Mos6502InstructionSet) {
        match instruction.opcode {
            Opcode::Mos6502(opcode) => {
                match opcode {
                    Mos6502Opcode::Adc => {
                        let value: u8 = self.load(&instruction);

                        self.adc(value);
                    }
                    Mos6502Opcode::Anc => {
                        let value: u8 = self.load(&instruction);

                        let result = self.state.a & value;
                        let result_bits = result.view_bits::<Lsb0>();

                        self.state.flags.carry = result_bits[7];
                        self.state.flags.negative = result_bits[7];
                        self.state.flags.zero = result == 0;

                        self.state.a = result;
                    }
                    Mos6502Opcode::And => {
                        let value: u8 = self.load(&instruction);

                        let result = self.state.a & value;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.a = result;
                    }
                    Mos6502Opcode::Arr => {
                        let value: u8 = self.load(&instruction);

                        let mut result = self.state.a & value;

                        let carry = self.state.flags.carry;
                        self.state.flags.carry = result.view_bits::<Lsb0>()[7];

                        result >>= 1;

                        let result_bits = result.view_bits_mut::<Lsb0>();
                        result_bits.set(7, carry);

                        self.state.flags.overflow = result_bits[1] != result_bits[0];
                        self.state.flags.negative = result_bits[0];
                        self.state.flags.zero = result == 0;

                        self.state.a = result;
                    }
                    Mos6502Opcode::Asl => {
                        let mut value: u8 = self.load(&instruction);
                        let value_bits = value.view_bits::<Lsb0>();

                        let carry = value_bits[7];
                        let negative = value_bits[6];

                        value <<= 1;

                        self.state.flags.carry = carry;
                        self.state.flags.negative = negative;
                        self.state.flags.zero = value == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            self.state.a = value;
                        } else {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Asr => {
                        let mut value: u8 = self.load(&instruction);
                        let value_bits = value.view_bits::<Lsb0>();

                        let carry = value_bits[7];

                        value = (self.state.a & value) >> 1;

                        self.state.flags.carry = carry;
                        self.state.flags.negative = false;
                        self.state.flags.zero = value == 0;

                        self.state.a = value;
                    }
                    Mos6502Opcode::Bcc => {
                        let value = self.load(&instruction) as i8;

                        if !self.state.flags.carry {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bcs => {
                        let value = self.load(&instruction) as i8;

                        if self.state.flags.carry {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Beq => {
                        let value = self.load(&instruction) as i8;

                        if self.state.flags.zero {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bit => {
                        let value: u8 = self.load(&instruction);
                        let value_bits = value.view_bits::<Lsb0>();

                        let result = self.state.a & value;

                        self.state.flags.negative = value_bits[7];
                        self.state.flags.overflow = value_bits[6];
                        self.state.flags.zero = result == 0;
                    }
                    Mos6502Opcode::Bmi => {
                        let value = self.load(&instruction) as i8;

                        if self.state.flags.negative {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bne => {
                        let value = self.load(&instruction) as i8;

                        if !self.state.flags.zero {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bpl => {
                        let value = self.load(&instruction) as i8;

                        if !self.state.flags.negative {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Brk => {
                        let new_stack = self.state.stack.wrapping_sub(2);
                        let program_bytes = (self.state.program.wrapping_add(2)).to_le_bytes();

                        let _ = self.address_space.write_le_value(
                            new_stack as usize + STACK_BASE_ADDRESS,
                            self.timestamp,
                            Some(&mut self.address_space_cache),
                            program_bytes[0],
                        );

                        let _ = self.address_space.write_le_value(
                            (new_stack.wrapping_add(1)) as usize + STACK_BASE_ADDRESS,
                            self.timestamp,
                            Some(&mut self.address_space_cache),
                            program_bytes[1],
                        );

                        // https://www.nesdev.org/wiki/Status_flags
                        let mut flags = self.state.flags;
                        flags.undocumented = true;
                        flags.break_ = true;
                        self.state.flags.interrupt_disable = true;

                        let new_stack = new_stack.wrapping_sub(1);

                        let _ = self.address_space.write_le_value(
                            new_stack as usize + STACK_BASE_ADDRESS,
                            self.timestamp,
                            Some(&mut self.address_space_cache),
                            flags.to_byte(),
                        );

                        let program = [
                            self.address_space
                                .read_le_value(
                                    INTERRUPT_VECTOR,
                                    self.timestamp,
                                    Some(&mut self.address_space_cache),
                                )
                                .unwrap_or_default(),
                            self.address_space
                                .read_le_value(
                                    INTERRUPT_VECTOR + 1,
                                    self.timestamp,
                                    Some(&mut self.address_space_cache),
                                )
                                .unwrap_or_default(),
                        ];
                        self.state.program = u16::from_le_bytes(program);

                        self.state.stack = new_stack;
                    }
                    Mos6502Opcode::Bvc => {
                        let value = self.load(&instruction) as i8;

                        if !self.state.flags.overflow {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Bvs => {
                        let value = self.load(&instruction) as i8;

                        if self.state.flags.overflow {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::ModifyProgramPointer(value));
                        }
                    }
                    Mos6502Opcode::Clc => {
                        self.state.flags.carry = false;
                    }
                    Mos6502Opcode::Cld => {
                        self.state.flags.decimal = false;
                    }
                    Mos6502Opcode::Cli => {
                        self.state.flags.interrupt_disable = false;
                    }
                    Mos6502Opcode::Clv => {
                        self.state.flags.overflow = false;
                    }
                    Mos6502Opcode::Cmp => {
                        let value: u8 = self.load(&instruction);

                        let result = self.state.a.wrapping_sub(value);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;
                        self.state.flags.carry = self.state.a >= value;
                    }
                    Mos6502Opcode::Cpx => {
                        let value: u8 = self.load(&instruction);

                        let result = self.state.x.wrapping_sub(value);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;
                        self.state.flags.carry = self.state.x >= value;
                    }
                    Mos6502Opcode::Cpy => {
                        let value: u8 = self.load(&instruction);

                        let result = self.state.y.wrapping_sub(value);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;
                        self.state.flags.carry = self.state.y >= value;
                    }
                    Mos6502Opcode::Dcp => todo!(),
                    Mos6502Opcode::Dec => {
                        let value: u8 = self.load(&instruction);

                        let result = value.wrapping_sub(1);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state
                            .execution_queue
                            .push_back(ExecutionStep::StoreData(result));
                    }
                    Mos6502Opcode::Dex => {
                        let result = self.state.x.wrapping_sub(1);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.x = result;
                    }
                    Mos6502Opcode::Dey => {
                        let result = self.state.y.wrapping_sub(1);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.y = result;
                    }
                    Mos6502Opcode::Eor => {
                        let value: u8 = self.load(&instruction);

                        let result = self.state.a ^ value;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.a = result;
                    }
                    Mos6502Opcode::Inc => {
                        let value: u8 = self.load(&instruction);

                        let result = value.wrapping_add(1);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state
                            .execution_queue
                            .push_back(ExecutionStep::StoreData(result));
                    }
                    Mos6502Opcode::Inx => {
                        let result = self.state.x.wrapping_add(1);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.x = result;
                    }
                    Mos6502Opcode::Iny => {
                        let result = self.state.y.wrapping_add(1);

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.y = result;
                    }
                    Mos6502Opcode::Isc => todo!(),
                    Mos6502Opcode::Jam => {
                        tracing::error!(
                            "The MOS 6502 processor inside this machine just jammed itself"
                        );

                        self.state.execution_queue.push_back(ExecutionStep::Jammed);
                    }
                    Mos6502Opcode::Jmp => {
                        let value = match instruction.addressing_mode {
                            Some(AddressingMode::Mos6502(
                                Mos6502AddressingMode::Absolute
                                | Mos6502AddressingMode::AbsoluteIndirect,
                            )) => self.state.address_bus,
                            _ => unreachable!(),
                        };

                        self.state.program = value;
                    }
                    Mos6502Opcode::Jsr => {
                        // We load the byte BEFORE the program counter
                        let program_bytes = (self.state.program.wrapping_sub(1)).to_le_bytes();

                        self.state.execution_queue.extend([
                            ExecutionStep::PushStack(program_bytes[1]),
                            ExecutionStep::PushStack(program_bytes[0]),
                            ExecutionStep::AddressBusToProgramPointer,
                        ]);
                    }
                    Mos6502Opcode::Las => todo!(),
                    Mos6502Opcode::Lax => {
                        let value: u8 = self.load(&instruction);

                        self.state.a = value;
                        self.state.x = value;
                    }
                    Mos6502Opcode::Lda => {
                        let value: u8 = self.load(&instruction);

                        self.state.flags.negative = value.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = value == 0;

                        self.state.a = value;
                    }
                    Mos6502Opcode::Ldx => {
                        let value: u8 = self.load(&instruction);

                        self.state.flags.negative = value.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = value == 0;

                        self.state.x = value;
                    }
                    Mos6502Opcode::Ldy => {
                        let value: u8 = self.load(&instruction);

                        self.state.flags.negative = value.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = value == 0;

                        self.state.y = value;
                    }
                    Mos6502Opcode::Lsr => {
                        let value: u8 = self.load(&instruction);
                        let value_bits = value.view_bits::<Lsb0>();

                        let carry = value_bits[0];
                        let value = value >> 1;

                        self.state.flags.negative = false;
                        self.state.flags.carry = carry;
                        self.state.flags.zero = value == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            self.state.a = value;
                        } else {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Nop => {
                        if instruction.addressing_mode.is_some() {
                            let _: u8 = self.load(&instruction);
                        }
                    }
                    Mos6502Opcode::Ora => {
                        let value: u8 = self.load(&instruction);

                        let result = self.state.a | value;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.a = result;
                    }
                    Mos6502Opcode::Pha => {
                        self.state
                            .execution_queue
                            .push_back(ExecutionStep::PushStack(self.state.a));
                    }
                    Mos6502Opcode::Php => {
                        let mut flags = self.state.flags;
                        // https://www.nesdev.org/wiki/Status_flags
                        flags.undocumented = true;
                        flags.break_ = true;

                        self.state
                            .execution_queue
                            .push_back(ExecutionStep::PushStack(flags.to_byte()));
                    }
                    Mos6502Opcode::Pla => {
                        self.state.stack = self.state.stack.wrapping_add(1);

                        self.state.a = self
                            .address_space
                            .read_le_value(
                                self.state.stack as usize + STACK_BASE_ADDRESS,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                            )
                            .unwrap_or_default();

                        self.state.flags.negative = self.state.a.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = self.state.a == 0;
                    }
                    Mos6502Opcode::Plp => {
                        self.state.stack = self.state.stack.wrapping_add(1);

                        let value = self
                            .address_space
                            .read_le_value(
                                self.state.stack as usize + STACK_BASE_ADDRESS,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                            )
                            .unwrap_or_default();

                        self.state.flags = FlagRegister::from_byte(value);
                    }
                    Mos6502Opcode::Rla => todo!(),
                    Mos6502Opcode::Rol => {
                        let mut value: u8 = self.load(&instruction);
                        let value_bits = value.view_bits::<Lsb0>();

                        let old_carry = self.state.flags.carry;
                        self.state.flags.carry = value_bits[7];
                        self.state.flags.negative = value_bits[6];
                        value <<= 1;
                        value.view_bits_mut::<Lsb0>().set(0, old_carry);

                        self.state.flags.zero = value == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            self.state.a = value;
                        } else {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Ror => {
                        let mut value: u8 = self.load(&instruction);
                        let value_bits = value.view_bits::<Lsb0>();

                        let old_carry = self.state.flags.carry;
                        self.state.flags.carry = value_bits[0];
                        self.state.flags.negative = old_carry;
                        value >>= 1;
                        value.view_bits_mut::<Lsb0>().set(7, old_carry);

                        self.state.flags.zero = value == 0;

                        if instruction.addressing_mode.unwrap()
                            == AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)
                        {
                            self.state.a = value;
                        } else {
                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::StoreData(value));
                        }
                    }
                    Mos6502Opcode::Rra => todo!(),
                    Mos6502Opcode::Rti => {
                        self.state.stack = self.state.stack.wrapping_add(1);
                        let flags = self
                            .address_space
                            .read_le_value::<u8>(
                                STACK_BASE_ADDRESS + self.state.stack as usize,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                            )
                            .unwrap_or_default();

                        self.state.flags = FlagRegister::from_byte(flags);

                        self.state.stack = self.state.stack.wrapping_add(1);
                        let program_pointer_low = self
                            .address_space
                            .read_le_value::<u8>(
                                STACK_BASE_ADDRESS + self.state.stack as usize,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                            )
                            .unwrap_or_default();

                        self.state.stack = self.state.stack.wrapping_add(1);
                        let program_pointer_high = self
                            .address_space
                            .read_le_value::<u8>(
                                STACK_BASE_ADDRESS + self.state.stack as usize,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                            )
                            .unwrap_or_default();

                        self.state.program =
                            u16::from_le_bytes([program_pointer_low, program_pointer_high]);
                    }
                    Mos6502Opcode::Rts => {
                        self.state.stack = self.state.stack.wrapping_add(1);
                        let program_pointer_low = self
                            .address_space
                            .read_le_value::<u8>(
                                STACK_BASE_ADDRESS + self.state.stack as usize,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                            )
                            .unwrap_or_default();

                        self.state.stack = self.state.stack.wrapping_add(1);
                        let program_pointer_high = self
                            .address_space
                            .read_le_value::<u8>(
                                STACK_BASE_ADDRESS + self.state.stack as usize,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                            )
                            .unwrap_or_default();

                        self.state.program =
                            u16::from_le_bytes([program_pointer_low, program_pointer_high]);
                        self.state.program = self.state.program.wrapping_add(1);
                    }
                    Mos6502Opcode::Sax => {
                        let value = self.state.a & self.state.x;

                        self.store(&instruction, value);
                    }
                    Mos6502Opcode::Sbc => {
                        let value: u8 = self.load(&instruction);

                        self.adc(!value);
                    }
                    Mos6502Opcode::Sbx => todo!(),
                    Mos6502Opcode::Sec => {
                        self.state.flags.carry = true;
                    }
                    Mos6502Opcode::Sed => {
                        self.state.flags.decimal = true;
                    }
                    Mos6502Opcode::Sei => {
                        self.state.flags.interrupt_disable = true;
                    }
                    Mos6502Opcode::Sha => todo!(),
                    Mos6502Opcode::Shs => todo!(),
                    Mos6502Opcode::Shx => todo!(),
                    Mos6502Opcode::Shy => todo!(),
                    Mos6502Opcode::Slo => todo!(),
                    Mos6502Opcode::Sre => todo!(),
                    Mos6502Opcode::Sta => {
                        self.store(&instruction, self.state.a);
                    }
                    Mos6502Opcode::Stx => {
                        self.store(&instruction, self.state.x);
                    }
                    Mos6502Opcode::Sty => {
                        self.store(&instruction, self.state.y);
                    }
                    Mos6502Opcode::Tax => {
                        let result = self.state.a;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.x = result;
                    }
                    Mos6502Opcode::Tay => {
                        let result = self.state.a;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.y = result;
                    }
                    Mos6502Opcode::Tsx => {
                        let result = self.state.stack;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.x = result;
                    }
                    Mos6502Opcode::Txa => {
                        let result = self.state.x;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.a = result;
                    }
                    Mos6502Opcode::Txs => {
                        self.state.stack = self.state.x;
                    }
                    Mos6502Opcode::Tya => {
                        let result = self.state.y;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = result == 0;

                        self.state.a = result;
                    }
                    Mos6502Opcode::Xaa => {
                        let value: u8 = self.load(&instruction);
                        let random_value: u8 = rand::random();

                        let result = (self.state.a & random_value) & self.state.x & value;

                        self.state.flags.negative = result.view_bits::<Lsb0>()[7];
                        self.state.flags.zero = value == 0;

                        self.state.a = result;
                    }
                }
            }
            Opcode::Wdc65C02(opcode) => match opcode {
                Wdc65C02Opcode::Bra => {
                    let value = self.load(&instruction) as i8;

                    self.state
                        .execution_queue
                        .push_back(ExecutionStep::ModifyProgramPointer(value));
                }
                Wdc65C02Opcode::Phx => {
                    self.state
                        .execution_queue
                        .push_back(ExecutionStep::PushStack(self.state.x));
                }
                Wdc65C02Opcode::Phy => {
                    self.state
                        .execution_queue
                        .push_back(ExecutionStep::PushStack(self.state.y));
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
    fn load(&mut self, instruction: &Mos6502InstructionSet) -> u8 {
        match instruction.addressing_mode {
            Some(AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)) => self.state.a,
            None => unreachable!(),
            _ => self
                .address_space
                .read_le_value(
                    self.state.address_bus as usize,
                    self.timestamp,
                    Some(&mut self.address_space_cache),
                )
                .unwrap_or_default(),
        }
    }

    #[inline]
    fn store(&mut self, instruction: &Mos6502InstructionSet, value: u8) {
        match instruction.addressing_mode {
            Some(AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator)) => {
                self.state.a = value;
            }
            None => {
                unreachable!()
            }
            _ => {
                let _ = self.address_space.write_le_value(
                    self.state.address_bus as usize,
                    self.timestamp,
                    Some(&mut self.address_space_cache),
                    value,
                );
            }
        }
    }

    #[inline]
    fn adc(&mut self, value: u8) {
        if self.state.flags.decimal && self.config.kind.supports_decimal() {
            todo!()
        } else {
            let carry = u8::from(self.state.flags.carry);

            let (first_operation_result, first_operation_carry) =
                self.state.a.overflowing_add(value);

            let (second_operation_result, second_operation_carry) =
                first_operation_result.overflowing_add(carry);

            let a_bits = self.state.a.view_bits::<Lsb0>();
            let value_bits = value.view_bits::<Lsb0>();
            let result_bits = second_operation_result.view_bits::<Lsb0>();

            self.state.flags.overflow =
                (a_bits[7] == value_bits[7]) && (a_bits[7] != result_bits[7]);
            self.state.flags.carry = first_operation_carry || second_operation_carry;
            self.state.flags.negative = result_bits[7];
            self.state.flags.zero = second_operation_result == 0;

            self.state.a = second_operation_result;
        }
    }
}
