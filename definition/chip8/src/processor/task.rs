use super::{
    Chip8KeyCode, Chip8Processor, Chip8ProcessorQuirks, ExecutionState,
    decoder::Chip8InstructionDecoder,
};
use crate::{
    CPU_ADDRESS_SPACE, Chip8Kind, audio::Chip8Audio, display::Chip8Display, timer::Chip8Timer,
};
use crossbeam::atomic::AtomicCell;
use multiemu_machine::{
    component::{RuntimeEssentials, component_ref::ComponentRef},
    input::virtual_gamepad::VirtualGamepad,
    processor::decoder::InstructionDecoder,
    scheduler::task::Task,
};
use std::{
    num::NonZero,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(crate) struct Chip8ProcessorTask {
    /// Instruction cache
    pub instruction_decoder: Chip8InstructionDecoder,
    /// Keypad virtual gamepad
    pub virtual_gamepad: Arc<VirtualGamepad>,
    /// Essential stuff the runtime provides
    pub essentials: Arc<RuntimeEssentials>,
    /// Quirks
    pub quirks: Arc<Chip8ProcessorQuirks>,
    // What chip8 mode we are currently in
    pub mode: Arc<AtomicCell<Chip8Kind>>,
    pub vsync_occurred: Arc<AtomicBool>,
    pub display_component: ComponentRef<Chip8Display>,
    pub timer_component: ComponentRef<Chip8Timer>,
    pub audio_component: ComponentRef<Chip8Audio>,
    #[cfg(jit)]
    pub jit: Option<
        multiemu_machine::processor::jit::InstructionJitExecutor<
            super::jit::Chip8InstructionTranslator,
        >,
    >,
}

impl Task<Chip8Processor> for Chip8ProcessorTask {
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
                                CPU_ADDRESS_SPACE,
                                self.essentials.memory_translation_table(),
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
                        if self.vsync_occurred.swap(false, Ordering::Relaxed) {
                            state.execution_state = ExecutionState::Normal;
                            break 'main;
                        }
                    }
                }
            }
        }
    }
}
