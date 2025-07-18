use super::{Chip8KeyCode, Chip8ProcessorConfig, ExecutionState, decoder::Chip8InstructionDecoder};
use crate::{Chip8Mode, display::SupportedGraphicsApiChip8Display, processor::Chip8Processor};
use multiemu_runtime::{
    component::ComponentRef, input::VirtualGamepad, memory::MemoryAccessTable,
    processor::InstructionDecoder, scheduler::Task,
};
use std::{
    num::NonZero,
    sync::{Arc, Mutex, atomic::Ordering},
};

pub(crate) struct Chip8ProcessorTask<G: SupportedGraphicsApiChip8Display> {
    /// Instruction cache
    pub instruction_decoder: Chip8InstructionDecoder,
    /// Reference to the component
    pub component: ComponentRef<Chip8Processor>,
    /// Keypad virtual gamepad
    pub virtual_gamepad: Arc<VirtualGamepad>,
    /// Essential stuff the runtime provides
    pub memory_translation_table: Arc<MemoryAccessTable>,
    // What chip8 mode we are currently in
    pub mode: Arc<Mutex<Chip8Mode>>,
    pub config: Chip8ProcessorConfig<G>,
}

impl<G: SupportedGraphicsApiChip8Display> Task for Chip8ProcessorTask<G> {
    fn run(&mut self, time_slice: NonZero<u32>) {
        let mut time_slice = time_slice.get();

        self.component
            .interact(|component| {
                let mut mode_guard = self.mode.lock().unwrap();
                let mut state_guard = component.state.lock().unwrap();

                while time_slice != 0 {
                    'main: {
                        match &state_guard.execution_state {
                            ExecutionState::Normal => {
                                let (decompiled_instruction, decompiled_instruction_length) = self
                                    .instruction_decoder
                                    .decode(
                                        state_guard.registers.program as usize,
                                        self.config.cpu_address_space,
                                        &self.memory_translation_table,
                                    )
                                    .expect("Failed to decode instruction");

                                state_guard.registers.program = state_guard
                                    .registers
                                    .program
                                    .wrapping_add(decompiled_instruction_length as u16);

                                tracing::trace!("Decoded instruction {:?}", decompiled_instruction);

                                self.interpret_instruction(
                                    &mut state_guard,
                                    &mut mode_guard,
                                    decompiled_instruction,
                                );
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
                                    state_guard.execution_state =
                                        ExecutionState::AwaitingKeyRelease {
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
                                        state_guard.registers.work_registers[register as usize] =
                                            key_code.0;
                                        state_guard.execution_state = ExecutionState::Normal;
                                        break 'main;
                                    }
                                }
                            }
                            ExecutionState::AwaitingVsync => {
                                let vsync_occured = self
                                    .config
                                    .display
                                    .interact(|display| {
                                        display.vsync_occurred.load(Ordering::Relaxed)
                                    })
                                    .unwrap();

                                if vsync_occured {
                                    state_guard.execution_state = ExecutionState::Normal;
                                    break 'main;
                                }
                            }
                            ExecutionState::Halted => {
                                // Do nothing
                            }
                        }
                    }

                    time_slice -= 1;
                }
            })
            .unwrap();
    }
}
