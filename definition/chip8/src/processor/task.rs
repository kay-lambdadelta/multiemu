use super::{Chip8KeyCode, Chip8ProcessorConfig, ExecutionState, decoder::Chip8InstructionDecoder};
use crate::{Chip8Kind, SupportedRenderApiChip8, processor::Chip8Processor};
use multiemu_runtime::{
    component::component_ref::ComponentRef,
    input::virtual_gamepad::VirtualGamepad,
    memory::memory_translation_table::MemoryTranslationTable,
    processor::decoder::InstructionDecoder,
    scheduler::{SchedulerHandle, Task, YieldReason},
};
use std::sync::{Arc, Mutex, atomic::Ordering};

pub(crate) struct Chip8ProcessorTask<R: SupportedRenderApiChip8> {
    /// Instruction cache
    pub instruction_decoder: Chip8InstructionDecoder,
    /// Reference to the component
    pub component: ComponentRef<Chip8Processor>,
    /// Keypad virtual gamepad
    pub virtual_gamepad: Arc<VirtualGamepad>,
    /// Essential stuff the runtime provides
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    // What chip8 mode we are currently in
    pub mode: Arc<Mutex<Chip8Kind>>,
    pub config: Chip8ProcessorConfig<R>,
}

impl<R: SupportedRenderApiChip8> Task for Chip8ProcessorTask<R> {
    fn run(self: Box<Self>, mut handle: SchedulerHandle) {
        let mut should_exit = false;

        while !should_exit {
            self.component
                .interact(|component| {
                    // Chip8 processor frequency is slow enough to do this
                    let mut state_guard = component.state.lock().unwrap();

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
                                        display.vsync_occurred.swap(false, Ordering::Relaxed)
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
                })
                .unwrap();

            handle.tick(|reason| {
                if reason == YieldReason::Exit {
                    should_exit = true
                }
            });
        }
    }
}
