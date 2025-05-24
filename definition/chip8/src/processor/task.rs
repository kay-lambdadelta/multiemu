use super::{
    Chip8KeyCode, Chip8Processor, Chip8ProcessorConfig, ExecutionState,
    decoder::Chip8InstructionDecoder,
};
use crate::{Chip8Kind, SupportedRenderApiChip8, display::Chip8DisplayBackend};
use crossbeam::atomic::AtomicCell;
use multiemu_machine::{
    input::virtual_gamepad::VirtualGamepad,
    memory::memory_translation_table::MemoryTranslationTable,
    processor::decoder::InstructionDecoder, task::Task,
};
use std::{
    num::NonZero,
    sync::{Arc, atomic::Ordering},
};

pub(crate) struct Chip8ProcessorTask<R: SupportedRenderApiChip8> {
    /// Instruction cache
    pub instruction_decoder: Chip8InstructionDecoder,
    /// Keypad virtual gamepad
    pub virtual_gamepad: Arc<VirtualGamepad>,
    /// Essential stuff the runtime provides
    pub memory_translation_table: MemoryTranslationTable,
    // What chip8 mode we are currently in
    pub mode: Arc<AtomicCell<Chip8Kind>>,
    pub config: Chip8ProcessorConfig<R>,
}

impl<R: SupportedRenderApiChip8> Task<Chip8Processor> for Chip8ProcessorTask<R> {
    fn run(&mut self, target: &Chip8Processor, period: NonZero<u32>) {
        let mut state = target.state.lock().unwrap();

        for _ in 0..period.get() {
            'main: {
                match &state.execution_state {
                    ExecutionState::Normal => {
                        let (decompiled_instruction, decompiled_instruction_length) = self
                            .instruction_decoder
                            .decode(
                                state.registers.program as usize,
                                self.config.cpu_address_space,
                                &self.memory_translation_table,
                            )
                            .expect("Failed to decode instruction");

                        state.registers.program = state
                            .registers
                            .program
                            .wrapping_add(decompiled_instruction_length as u16);

                        tracing::trace!("Decoded instruction {:?}", decompiled_instruction);

                        self.interpret_instruction(&mut state, decompiled_instruction);
                    }
                    ExecutionState::AwaitingKeyPress { register } => {
                        // FIXME: A allocation every cycle isn't a good idea
                        let mut pressed = Vec::new();

                        // Go through every chip8 key
                        for key in 0x0..0xf {
                            let keycode = Chip8KeyCode(key);

                            if self
                                .virtual_gamepad
                                .get(keycode.try_into().unwrap())
                                .as_digital(None)
                            {
                                pressed.push(keycode);
                            }
                        }

                        if !pressed.is_empty() {
                            state.execution_state = ExecutionState::AwaitingKeyRelease {
                                register: *register,
                                keys: pressed,
                            };

                            break 'main;
                        }
                    }
                    ExecutionState::AwaitingKeyRelease { register, keys } => {
                        for key_code in keys {
                            if !self
                                .virtual_gamepad
                                .get((*key_code).try_into().unwrap())
                                .as_digital(None)
                            {
                                let register = *register;
                                state.registers.work_registers[register as usize] = key_code.0;
                                state.execution_state = ExecutionState::Normal;
                                break 'main;
                            }
                        }
                    }
                    ExecutionState::AwaitingVsync => {
                        let vsync_occured = self
                            .config
                            .display
                            .interact(|display| {
                                display.vsync_occurred.swap(false, Ordering::Relaxed)
                            })
                            .unwrap();

                        if vsync_occured {
                            state.execution_state = ExecutionState::Normal;
                            break 'main;
                        }
                    }
                    ExecutionState::Halted => {
                        // Do nothing
                    }
                }
            }
        }
    }
}
