use super::{Chip8KeyCode, Chip8ProcessorConfig, ExecutionState, decoder::Chip8InstructionDecoder};
use crate::{
    Chip8Mode,
    audio::Chip8Audio,
    display::{Chip8Display, SupportedGraphicsApiChip8Display},
    processor::Chip8Processor,
    timer::Chip8Timer,
};
use multiemu_runtime::{
    component::ComponentHandle, machine::virtual_gamepad::VirtualGamepad,
    memory::MemoryAccessTable, processor::InstructionDecoder, scheduler::Task,
};
use std::{
    num::NonZero,
    sync::{Arc, Mutex},
};

pub(crate) struct Driver<G: SupportedGraphicsApiChip8Display> {
    /// Instruction cache
    pub instruction_decoder: Chip8InstructionDecoder,
    /// Keypad virtual gamepad
    pub virtual_gamepad: Arc<VirtualGamepad>,
    /// Essential stuff the runtime provides
    pub memory_access_table: Arc<MemoryAccessTable>,
    // What chip8 mode we are currently in
    pub mode: Arc<Mutex<Chip8Mode>>,
    pub display: ComponentHandle<Chip8Display<G>>,
    pub audio: ComponentHandle<Chip8Audio>,
    pub timer: ComponentHandle<Chip8Timer>,
    pub config: Chip8ProcessorConfig<G>,
}

impl<G: SupportedGraphicsApiChip8Display> Task<Chip8Processor> for Driver<G> {
    fn run(&mut self, component: &mut Chip8Processor, time_slice: NonZero<u32>) {
        let mut time_slice = time_slice.get();

        let mut mode_guard = self.mode.lock().unwrap();

        while time_slice != 0 {
            'main: {
                match &component.state.execution_state {
                    ExecutionState::Normal => {
                        let (decompiled_instruction, decompiled_instruction_length) = self
                            .instruction_decoder
                            .decode(
                                component.state.registers.program as usize,
                                self.config.cpu_address_space,
                                &self.memory_access_table,
                            )
                            .expect("Failed to decode instruction");

                        component.state.registers.program = component
                            .state
                            .registers
                            .program
                            .wrapping_add(u16::from(decompiled_instruction_length));

                        tracing::trace!("Decoded instruction {:?}", decompiled_instruction);

                        self.interpret_instruction(
                            &mut component.state,
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
                            component.state.execution_state = ExecutionState::AwaitingKeyRelease {
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
                                component.state.registers.work_registers[register as usize] =
                                    key_code.0;
                                component.state.execution_state = ExecutionState::Normal;
                                break 'main;
                            }
                        }
                    }
                    ExecutionState::AwaitingVsync => {
                        let vsync_occured =
                            self.display.interact(|component| component.vsync_occurred);

                        if vsync_occured {
                            component.state.execution_state = ExecutionState::Normal;
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
    }
}
