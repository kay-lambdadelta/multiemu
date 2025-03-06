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

impl M6502Task {
    pub(super) fn interpret_instruction(
        &self,
        state: &mut ProcessorState,
        instruction: M6502InstructionSet,
    ) {
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

                    state.registers.accumulator = second_operation_result;
                }

                state.set_accumulator_flags();
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
                state
                    .registers
                    .flags
                    .set(FlagRegister::Carry, result.view_bits::<Msb0>()[0]);

                state.registers.accumulator = result;
                state.set_accumulator_flags();
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

                state.registers.accumulator &= value;
                state.set_accumulator_flags();
            }
            M6502InstructionSetSpecifier::Arr => {
                let value = load_m6502_addressing_modes!(
                    instruction,
                    state.registers,
                    memory_translation_table,
                    self.config.assigned_address_space,
                    [Immediate]
                );

                let result = state.registers.accumulator & value;

                let carry = state.registers.flags.contains(FlagRegister::Carry);
                state
                    .registers
                    .flags
                    .set(FlagRegister::Carry, result.view_bits::<Msb0>()[0]);

                let mut result = result >> 1;
                result.view_bits_mut::<Msb0>().set(0, carry);
                let result_bits = result.view_bits::<Msb0>();

                state
                    .registers
                    .flags
                    .set(FlagRegister::Overflow, result_bits[1] != result_bits[0]);

                state.registers.accumulator = result;
                state.set_accumulator_flags();
            }
            M6502InstructionSetSpecifier::Asl => todo!(),
            M6502InstructionSetSpecifier::Asr => todo!(),
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
            M6502InstructionSetSpecifier::Brk => todo!(),
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
            M6502InstructionSetSpecifier::Dex => todo!(),
            M6502InstructionSetSpecifier::Dey => todo!(),
            M6502InstructionSetSpecifier::Eor => todo!(),
            M6502InstructionSetSpecifier::Inc => todo!(),
            M6502InstructionSetSpecifier::Inx => todo!(),
            M6502InstructionSetSpecifier::Iny => todo!(),
            M6502InstructionSetSpecifier::Isc => todo!(),
            M6502InstructionSetSpecifier::Jam => todo!(),
            M6502InstructionSetSpecifier::Jmp => todo!(),
            M6502InstructionSetSpecifier::Jsr => todo!(),
            M6502InstructionSetSpecifier::Las => todo!(),
            M6502InstructionSetSpecifier::Lax => todo!(),
            M6502InstructionSetSpecifier::Lda => todo!(),
            M6502InstructionSetSpecifier::Ldx => todo!(),
            M6502InstructionSetSpecifier::Ldy => todo!(),
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

                state.registers.accumulator |= value;
                state.set_accumulator_flags();
            }
            M6502InstructionSetSpecifier::Pha => {
                let _ = memory_translation_table.write(
                    state.registers.stack_pointer as usize,
                    self.config.assigned_address_space,
                    &state.registers.accumulator.to_le_bytes(),
                );

                state.registers.stack_pointer = state.registers.stack_pointer.wrapping_sub(1);
            }
            M6502InstructionSetSpecifier::Php => {
                let mut flags = state.registers.flags;
                // https://www.nesdev.org/wiki/Status_flags
                flags.insert(FlagRegister::__Unused);

                let _ = memory_translation_table.write(
                    state.registers.stack_pointer as usize,
                    self.config.assigned_address_space,
                    &flags.bits().to_ne_bytes(),
                );

                state.registers.stack_pointer = state.registers.stack_pointer.wrapping_sub(1);
            }
            M6502InstructionSetSpecifier::Pla => {
                state.registers.stack_pointer = state.registers.stack_pointer.wrapping_add(1);

                let mut value = 0;

                let _ = memory_translation_table.read(
                    state.registers.stack_pointer as usize,
                    self.config.assigned_address_space,
                    std::array::from_mut(&mut value),
                );

                state.registers.accumulator = value;
                state.set_accumulator_flags();
            }
            M6502InstructionSetSpecifier::Plp => {
                state.registers.stack_pointer = state.registers.stack_pointer.wrapping_add(1);

                let mut value = 0;

                let _ = memory_translation_table.read(
                    state.registers.stack_pointer as usize,
                    self.config.assigned_address_space,
                    std::array::from_mut(&mut value),
                );

                state.registers.flags = FlagRegister::from_bits(value).unwrap();
            }
            M6502InstructionSetSpecifier::Rla => todo!(),
            M6502InstructionSetSpecifier::Rol => todo!(),
            M6502InstructionSetSpecifier::Ror => todo!(),
            M6502InstructionSetSpecifier::Rra => todo!(),
            M6502InstructionSetSpecifier::Rti => todo!(),
            M6502InstructionSetSpecifier::Rts => todo!(),
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
            M6502InstructionSetSpecifier::Sta => todo!(),
            M6502InstructionSetSpecifier::Stx => todo!(),
            M6502InstructionSetSpecifier::Sty => todo!(),
            M6502InstructionSetSpecifier::Tax => todo!(),
            M6502InstructionSetSpecifier::Tay => todo!(),
            M6502InstructionSetSpecifier::Tsx => todo!(),
            M6502InstructionSetSpecifier::Txa => todo!(),
            M6502InstructionSetSpecifier::Txs => todo!(),
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
