use super::{
    FlagRegister, ProcessorState,
    instruction::{M6502InstructionSet, M6502InstructionSetSpecifier},
};
use crate::{
    instruction::AddressingMode, load_m6502_addressing_modes, store_m6502_addressing_modes,
    task::M6502Task,
};
use bitvec::{prelude::Msb0, view::BitView};
use enumflags2::BitFlag;

mod load;
mod store;

const STACK_BASE_ADDRESS: usize = 0x0100;
const INTERRUPT_VECTOR: usize = 0xfffe;

// NOTE: https://www.pagetable.com/c64ref/6502

impl M6502Task {
    pub(super) fn interpret_instruction(
        &self,
        state: &mut ProcessorState,
        instruction: M6502InstructionSet,
    ) {
        tracing::debug!("Interpreting instruction: {:x?}", instruction,);

        let memory_translation_table = self.essentials.memory_translation_table();

        match instruction.specifier {
            M6502InstructionSetSpecifier::Adc => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [
                        Immediate,
                        Absolute,
                        XIndexedAbsolute,
                        YIndexedAbsolute,
                        ZeroPage,
                        XIndexedZeroPage,
                        XIndexedZeroPageIndirect,
                        ZeroPageIndirectYIndexed
                    ]
                );

                if state.registers.flags.contains(FlagRegister::Decimal)
                    && self.config.kind.supports_decimal()
                {
                } else {
                    let carry = state.registers.flags.contains(FlagRegister::Carry) as u8;

                    let (first_operation_result, first_operation_overflow) =
                        state.registers.accumulator.overflowing_add(value);

                    let (second_operation_result, second_operation_overflow) =
                        first_operation_result.overflowing_add(carry);

                    state.registers.flags.set(
                        FlagRegister::Overflow,
                        // If it overflowed at any point this is set
                        first_operation_overflow || second_operation_overflow,
                    );

                    state.registers.flags.set(
                        FlagRegister::Carry,
                        first_operation_overflow || second_operation_overflow,
                    );

                    state.registers.flags.set(
                        FlagRegister::Negative,
                        second_operation_result.view_bits::<Msb0>()[0],
                    );
                    state
                        .registers
                        .flags
                        .set(FlagRegister::Zero, second_operation_result == 0);

                    state.registers.accumulator = second_operation_result;
                }
            }
            M6502InstructionSetSpecifier::Anc => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [Immediate]
                );

                let result = state.registers.accumulator & value;

                state.registers.flags.set(
                    FlagRegister::Carry | FlagRegister::Negative,
                    result.view_bits::<Msb0>()[0],
                );
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.accumulator = result;
            }
            M6502InstructionSetSpecifier::And => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [
                        Immediate,
                        Absolute,
                        XIndexedAbsolute,
                        YIndexedAbsolute,
                        ZeroPage,
                        XIndexedZeroPage,
                        XIndexedZeroPageIndirect,
                        ZeroPageIndirectYIndexed
                    ]
                );

                let result = state.registers.accumulator & value;

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.accumulator = result;
            }
            M6502InstructionSetSpecifier::Arr => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [Immediate]
                );

                let mut result = state.registers.accumulator & value;

                let carry = state.registers.flags.contains(FlagRegister::Carry);
                state
                    .registers
                    .flags
                    .set(FlagRegister::Carry, result.view_bits::<Msb0>()[0]);

                result >>= 1;

                let result_bits = result.view_bits_mut::<Msb0>();
                result_bits.set(0, carry);

                state
                    .registers
                    .flags
                    .set(FlagRegister::Overflow, result_bits[1] != result_bits[0]);
                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result_bits[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.accumulator = result;
            }
            M6502InstructionSetSpecifier::Asl => {
                let mut value =
                    if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                        state.registers.accumulator
                    } else {
                        load_m6502_addressing_modes!(
                            instruction,
                            state.registers,
                            memory_translation_table,
                            self.config.assigned_address_space,
                            [Absolute, XIndexedAbsolute, ZeroPage, XIndexedZeroPage]
                        )
                    };

                let value_bits = value.view_bits::<Msb0>();

                let carry = value_bits[0];
                let negative = value_bits[1];
                value <<= 1;

                state.registers.flags.set(FlagRegister::Carry, carry);
                state.registers.flags.set(FlagRegister::Negative, negative);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.registers.accumulator = value;
                } else {
                    store_m6502_addressing_modes!(
                        instruction,
                        state.registers,
                        memory_translation_table,
                        self.config.assigned_address_space,
                        value,
                        [Absolute, XIndexedAbsolute, ZeroPage, XIndexedZeroPage]
                    );
                }
            }
            M6502InstructionSetSpecifier::Asr => {
                let mut value =
                    if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                        state.registers.accumulator
                    } else {
                        load_m6502_addressing_modes!(
                            instruction,
                            state.registers,
                            memory_translation_table,
                            self.config.assigned_address_space,
                            [Absolute, XIndexedAbsolute, ZeroPage, XIndexedZeroPage]
                        )
                    };

                let value_bits = value.view_bits::<Msb0>();

                let carry = value_bits[0];
                let negative = value_bits[1];
                value >>= 1;

                state.registers.flags.set(FlagRegister::Carry, carry);
                state.registers.flags.set(FlagRegister::Negative, negative);

                if instruction.addressing_mode.unwrap() == AddressingMode::Accumulator {
                    state.registers.accumulator = value;
                } else {
                    store_m6502_addressing_modes!(
                        instruction,
                        state.registers,
                        memory_translation_table,
                        self.config.assigned_address_space,
                        value,
                        [Absolute, XIndexedAbsolute, ZeroPage, XIndexedZeroPage]
                    );
                }
            }
            M6502InstructionSetSpecifier::Bcc => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if !state.registers.flags.contains(FlagRegister::Carry) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Bcs => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if state.registers.flags.contains(FlagRegister::Carry) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Beq => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if state.registers.flags.contains(FlagRegister::Zero) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Bit => todo!(),
            M6502InstructionSetSpecifier::Bmi => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if state.registers.flags.contains(FlagRegister::Negative) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Bne => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if !state.registers.flags.contains(FlagRegister::Zero) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Bpl => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if !state.registers.flags.contains(FlagRegister::Negative) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Brk => {
                let new_stack = state.registers.stack.wrapping_sub(2);

                let _ = memory_translation_table.write(
                    new_stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    &state.registers.program.to_le_bytes(),
                );

                // https://www.nesdev.org/wiki/Status_flags
                let mut flags = state.registers.flags;
                flags.insert(FlagRegister::__Unused);
                flags.insert(FlagRegister::Break);

                let new_stack = new_stack.wrapping_sub(1);

                let _ = memory_translation_table.write(
                    new_stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    std::array::from_ref(&flags.bits()),
                );

                let mut interrupt_location = [0; 2];

                let _ = memory_translation_table.read(
                    INTERRUPT_VECTOR,
                    self.config.assigned_address_space,
                    &mut interrupt_location,
                );

                state.registers.program = u16::from_le_bytes(interrupt_location);
                state.registers.stack = new_stack;
            }
            M6502InstructionSetSpecifier::Bvc => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if !state.registers.flags.contains(FlagRegister::Overflow) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Bvs => {
                let value = match instruction.addressing_mode {
                    Some(AddressingMode::Relative(value)) => value,
                    _ => unreachable!(),
                };

                if state.registers.flags.contains(FlagRegister::Overflow) {
                    state.registers.program =
                        state.registers.program.wrapping_add_signed(value as i16);
                }
            }
            M6502InstructionSetSpecifier::Clc => {
                state.registers.flags.remove(FlagRegister::Carry);
            }
            M6502InstructionSetSpecifier::Cld => {
                state.registers.flags.remove(FlagRegister::Decimal);
            }
            M6502InstructionSetSpecifier::Cli => {
                state.registers.flags.remove(FlagRegister::InterruptDisable);
            }
            M6502InstructionSetSpecifier::Clv => {
                state.registers.flags.remove(FlagRegister::Overflow);
            }
            M6502InstructionSetSpecifier::Cmp => todo!(),
            M6502InstructionSetSpecifier::Cpx => todo!(),
            M6502InstructionSetSpecifier::Cpy => todo!(),
            M6502InstructionSetSpecifier::Dcp => todo!(),
            M6502InstructionSetSpecifier::Dec => todo!(),
            M6502InstructionSetSpecifier::Dex => {
                let result = state.registers.index_registers[0].wrapping_sub(1);

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.index_registers[0] = result;
            }
            M6502InstructionSetSpecifier::Dey => {
                let result = state.registers.index_registers[1].wrapping_sub(1);

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.index_registers[1] = result;
            }
            M6502InstructionSetSpecifier::Eor => todo!(),
            M6502InstructionSetSpecifier::Inc => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [Absolute, XIndexedAbsolute, ZeroPage, XIndexedZeroPage]
                );

                let result = value.wrapping_add(1);

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                store_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    result,
                    [Absolute, XIndexedAbsolute, ZeroPage, XIndexedZeroPage]
                );
            }
            M6502InstructionSetSpecifier::Inx => {
                let result = state.registers.index_registers[0].wrapping_add(1);

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.index_registers[0] = result;
            }
            M6502InstructionSetSpecifier::Iny => {
                let result = state.registers.index_registers[1].wrapping_add(1);

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.index_registers[1] = result;
            }
            M6502InstructionSetSpecifier::Isc => todo!(),
            M6502InstructionSetSpecifier::Jam => todo!(),
            M6502InstructionSetSpecifier::Jmp => todo!(),
            M6502InstructionSetSpecifier::Jsr => {
                let Some(AddressingMode::Absolute(address)) = instruction.addressing_mode else {
                    unreachable!()
                };

                let new_stack = state.registers.stack.wrapping_sub(2);

                let _ = memory_translation_table.write(
                    new_stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    &state.registers.program.to_le_bytes(),
                );

                state.registers.program = address;
                state.registers.stack = new_stack;
            }
            M6502InstructionSetSpecifier::Las => todo!(),
            M6502InstructionSetSpecifier::Lax => todo!(),
            M6502InstructionSetSpecifier::Lda => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [
                        Immediate,
                        Absolute,
                        XIndexedAbsolute,
                        YIndexedAbsolute,
                        ZeroPage,
                        XIndexedZeroPage,
                        XIndexedZeroPageIndirect,
                        ZeroPageIndirectYIndexed
                    ]
                );

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, value.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, value == 0);

                state.registers.accumulator = value;
            }
            M6502InstructionSetSpecifier::Ldx => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [
                        Immediate,
                        Absolute,
                        YIndexedAbsolute,
                        ZeroPage,
                        YIndexedZeroPage
                    ]
                );

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, value.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, value == 0);

                state.registers.index_registers[0] = value;
            }
            M6502InstructionSetSpecifier::Ldy => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [
                        Immediate,
                        Absolute,
                        XIndexedAbsolute,
                        ZeroPage,
                        XIndexedZeroPage
                    ]
                );

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, value.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, value == 0);

                state.registers.index_registers[1] = value;
            }
            M6502InstructionSetSpecifier::Lsr => todo!(),
            M6502InstructionSetSpecifier::Nop => {
                if instruction.addressing_mode.is_some() {
                    let _ = load_m6502_addressing_modes!(
                        instruction,
                        state.registers,
                        memory_translation_table,
                        self.config.assigned_address_space,
                        [
                            Immediate,
                            Absolute,
                            XIndexedAbsolute,
                            ZeroPage,
                            XIndexedZeroPage
                        ]
                    );
                }
            }
            M6502InstructionSetSpecifier::Ora => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [
                        Immediate,
                        Absolute,
                        XIndexedAbsolute,
                        YIndexedAbsolute,
                        ZeroPage,
                        XIndexedZeroPage,
                        XIndexedZeroPageIndirect,
                        ZeroPageIndirectYIndexed
                    ]
                );

                let result = state.registers.accumulator | value;

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.accumulator = result;
            }
            M6502InstructionSetSpecifier::Pha => {
                let new_stack = state.registers.stack.wrapping_sub(1);

                let _ = memory_translation_table.write(
                    new_stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    &state.registers.accumulator.to_le_bytes(),
                );

                state.registers.stack = new_stack;
            }
            M6502InstructionSetSpecifier::Php => {
                let mut flags = state.registers.flags;
                // https://www.nesdev.org/wiki/Status_flags
                flags.insert(FlagRegister::__Unused);
                flags.insert(FlagRegister::Break);

                let new_stack = state.registers.stack.wrapping_sub(1);

                let _ = memory_translation_table.write(
                    new_stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    &flags.bits().to_ne_bytes(),
                );

                state.registers.stack = new_stack;
            }
            M6502InstructionSetSpecifier::Pla => {
                let mut value = 0;

                let _ = memory_translation_table.read(
                    state.registers.stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    std::array::from_mut(&mut value),
                );

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, value.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, value == 0);

                state.registers.accumulator = value;
                state.registers.stack = state.registers.stack.wrapping_add(1);
            }
            M6502InstructionSetSpecifier::Plp => {
                let mut value = 0;

                let _ = memory_translation_table.read(
                    state.registers.stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    std::array::from_mut(&mut value),
                );

                state.registers.flags = FlagRegister::from_bits(value).unwrap();
                state.registers.stack = state.registers.stack.wrapping_add(1);
            }
            M6502InstructionSetSpecifier::Rla => todo!(),
            M6502InstructionSetSpecifier::Rol => todo!(),
            M6502InstructionSetSpecifier::Ror => todo!(),
            M6502InstructionSetSpecifier::Rra => todo!(),
            M6502InstructionSetSpecifier::Rti => todo!(),
            M6502InstructionSetSpecifier::Rts => {
                let mut address = [0; 2];

                let _ = memory_translation_table.read(
                    state.registers.stack as usize + STACK_BASE_ADDRESS,
                    self.config.assigned_address_space,
                    &mut address,
                );

                state.registers.program = u16::from_le_bytes(address);
                state.registers.stack = state.registers.stack.wrapping_add(2);
            }
            M6502InstructionSetSpecifier::Sax => {
                let value = state.registers.accumulator & state.registers.index_registers[0];

                store_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    value,
                    [
                        Absolute,
                        ZeroPage,
                        YIndexedZeroPage,
                        XIndexedZeroPageIndirect
                    ]
                );
            }
            M6502InstructionSetSpecifier::Sbc => todo!(),
            M6502InstructionSetSpecifier::Sbx => todo!(),
            M6502InstructionSetSpecifier::Sec => {
                state.registers.flags.insert(FlagRegister::Carry);
            }
            M6502InstructionSetSpecifier::Sed => {
                state.registers.flags.insert(FlagRegister::Decimal);
            }
            M6502InstructionSetSpecifier::Sei => {
                state.registers.flags.insert(FlagRegister::InterruptDisable);
            }
            M6502InstructionSetSpecifier::Sha => todo!(),
            M6502InstructionSetSpecifier::Shs => todo!(),
            M6502InstructionSetSpecifier::Shx => todo!(),
            M6502InstructionSetSpecifier::Shy => todo!(),
            M6502InstructionSetSpecifier::Slo => todo!(),
            M6502InstructionSetSpecifier::Sre => todo!(),
            M6502InstructionSetSpecifier::Sta => {
                let value = state.registers.accumulator;

                store_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    value,
                    [
                        Absolute,
                        XIndexedAbsolute,
                        YIndexedAbsolute,
                        ZeroPage,
                        XIndexedZeroPage,
                        XIndexedZeroPageIndirect,
                        ZeroPageIndirectYIndexed
                    ]
                );
            }
            M6502InstructionSetSpecifier::Stx => todo!(),
            M6502InstructionSetSpecifier::Sty => todo!(),
            M6502InstructionSetSpecifier::Tax => todo!(),
            M6502InstructionSetSpecifier::Tay => todo!(),
            M6502InstructionSetSpecifier::Tsx => todo!(),
            M6502InstructionSetSpecifier::Txa => {
                let result = state.registers.index_registers[0];

                state
                    .registers
                    .flags
                    .set(FlagRegister::Negative, result.view_bits::<Msb0>()[0]);
                state.registers.flags.set(FlagRegister::Zero, result == 0);

                state.registers.accumulator = result;
            }
            M6502InstructionSetSpecifier::Txs => {
                state.registers.stack = state.registers.index_registers[0];
            }
            M6502InstructionSetSpecifier::Tya => todo!(),
            M6502InstructionSetSpecifier::Xaa => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [Immediate]
                );
            }
        }
    }
}
