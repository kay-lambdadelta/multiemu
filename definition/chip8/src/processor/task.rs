use super::{
    decoder::Chip8InstructionDecoder, Chip8KeyCode, Chip8Processor, Chip8ProcessorConfig,
    ExecutionState, ProcessorState,
};
use crate::{audio::Chip8Audio, display::Chip8Display, timer::Chip8Timer};
use multiemu_input::virtual_gamepad::VirtualGamepad;
use multiemu_machine::{
    component::{component_ref::ComponentRef, RuntimeEssentials},
    processor::{cache::InstructionCache, decoder::InstructionDecoder},
    scheduler::task::Task,
};
use std::sync::{Arc, RwLock};

pub(crate) struct Chip8ProcessorTask {
    /// Configuration this processor was created with
    pub config: Chip8ProcessorConfig,
    pub display_component: ComponentRef<Chip8Display>,
    pub timer_component: ComponentRef<Chip8Timer>,
    pub audio_component: ComponentRef<Chip8Audio>,
    /// parts of the cpu that actually change over execution
    pub state: Arc<RwLock<ProcessorState>>,
    /// Instruction cache
    pub instruction_cache: InstructionCache<Chip8InstructionDecoder>,
    /// Keypad virtual gamepad
    pub virtual_gamepad: Option<Arc<VirtualGamepad>>,
    /// Stored components
    pub essentials: Arc<RuntimeEssentials>,
    #[cfg(jit)]
    pub jit: Option<
        multiemu_machine::processor::jit::InstructionJitExecutor<
            super::jit::Chip8InstructionTranslator,
        >,
    >,
}

impl Task<Chip8Processor> for Chip8ProcessorTask {
    fn run(&mut self, _target: &Chip8Processor, period: u64) {
        let mut state = self.state.write().unwrap();

        for _ in 0..period {
            match &state.execution_state {
                ExecutionState::Normal => {
                    let (decompiled_instruction, decompiled_instruction_length) = self
                        .instruction_cache
                        .decode(
                            state.registers.program as usize,
                            &self.essentials.memory_translation_table(),
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
                    let gamepad = self.virtual_gamepad.as_ref().unwrap();

                    // Go through every chip8 key
                    for key in 0x0..0xf {
                        let keycode = Chip8KeyCode(key);

                        if gamepad.get(keycode.try_into().unwrap()).as_digital(None) {
                            pressed.push(keycode);
                        }
                    }

                    if !pressed.is_empty() {
                        state.execution_state = ExecutionState::AwaitingKeyRelease {
                            register: *register,
                            keys: pressed,
                        }
                    }
                }
                ExecutionState::AwaitingKeyRelease { register, keys } => {
                    let gamepad = self.virtual_gamepad.as_ref().unwrap();

                    for key_code in keys {
                        if !gamepad
                            .get((*key_code).try_into().unwrap())
                            .as_digital(None)
                        {
                            let register = *register;
                            state.registers.work_registers[register as usize] = key_code.0;
                            state.execution_state = ExecutionState::Normal;
                            break;
                        }
                    }
                }
            }
        }
    }
}
