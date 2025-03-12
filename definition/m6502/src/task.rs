use crate::{
    ExecutionMode, M6502, M6502Config, M6502Registers, RESET_VECTOR,
    decoder::M6502InstructionDecoder,
};
use multiemu_machine::{
    component::RuntimeEssentials, processor::decoder::InstructionDecoder, scheduler::task::Task,
};
use std::sync::Arc;

pub struct M6502Task {
    pub essentials: Arc<RuntimeEssentials>,
    pub instruction_decoder: M6502InstructionDecoder,
    pub config: Arc<M6502Config>,
}

impl Task<M6502> for M6502Task {
    fn run(&mut self, target: &M6502, period: u64) {
        for _ in 0..period {
            'main: {
                let mut state = target.state.lock().unwrap();

                state.cycle_counter = state.cycle_counter.saturating_sub(1);
                if state.cycle_counter == 0 {
                    match &state.execution_mode {
                        ExecutionMode::Normal => {
                            let (decompiled_instruction, decompiled_instruction_length) = self
                                .instruction_decoder
                                .decode(
                                    state.registers.program as usize,
                                    self.config.assigned_address_space,
                                    self.essentials.memory_translation_table(),
                                )
                                .expect("Failed to decode instruction");

                            state.registers.program = state
                                .registers
                                .program
                                .wrapping_add(decompiled_instruction_length as u16);

                            self.interpret_instruction(&mut state, decompiled_instruction);
                        }
                        ExecutionMode::Jammed => {}
                        ExecutionMode::Startup => {
                            let program = self.load_from_reset_vector();

                            state.registers = M6502Registers {
                                program,
                                ..self.config.initial_state
                            };
                            state.execution_mode = ExecutionMode::Normal;
                            state.cycle_counter = 0;

                            break 'main;
                        }
                        ExecutionMode::Reset => {
                            let program = self.load_from_reset_vector();

                            state.registers.program = program;
                            state.registers.stack = self.config.initial_state.stack;
                            state.execution_mode = ExecutionMode::Normal;
                            state.cycle_counter = 0;

                            break 'main;
                        }
                    }
                }
            }
        }
    }
}

impl M6502Task {
    fn load_from_reset_vector(&mut self) -> u16 {
        let mut program = [0; 2];
        let _ = self.essentials.memory_translation_table().read(
            RESET_VECTOR,
            self.config.assigned_address_space,
            &mut program,
        );

        u16::from_le_bytes(program)
    }
}
