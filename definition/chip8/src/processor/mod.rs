use crate::Chip8InstructionDecoder;

use super::Chip8Kind;
use arrayvec::ArrayVec;
use input::{default_bindings, present_inputs, Chip8KeyCode, CHIP8_KEYPAD_GAMEPAD_TYPE};
use instruction::Register;
use multiemu_config::ProcessorExecutionMode;
use multiemu_input::virtual_gamepad::VirtualGamepadMetadata;
use multiemu_machine::builder::ComponentBuilder;
use multiemu_machine::component::{Component, ComponentId, FromConfig, RuntimeEssentials};
use multiemu_machine::processor::cache::InstructionCache;
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use task::Chip8ProcessorTask;

pub mod decoder;
mod input;
mod instruction;
mod interpret;
#[cfg(jit)]
mod jit;
mod task;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
enum ExecutionState {
    Normal,
    AwaitingKeyPress {
        register: Register,
    },
    // KeyQuery does not return on key press but on key release, contrary to some documentation
    AwaitingKeyRelease {
        register: Register,
        keys: Vec<Chip8KeyCode>,
    },
}

// This is extremely complex because the chip8 cpu has a lot of non cpu machinery

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(jit, repr(C))]
struct Chip8ProcessorRegisters {
    work_registers: [u8; 16],
    index: u16,
    program: u16,
}

impl Default for Chip8ProcessorRegisters {
    fn default() -> Self {
        Self {
            work_registers: [0; 16],
            index: 0,
            program: 0x200,
        }
    }
}

#[derive(Debug)]
pub struct Chip8ProcessorConfig {
    pub frequency: Ratio<u64>,
    pub kind: Chip8Kind,
    pub display_component_id: ComponentId,
    pub timer_component_id: ComponentId,
    pub audio_component_id: ComponentId,
}

#[derive(Debug)]
#[cfg_attr(jit, repr(C))]
pub struct ProcessorState {
    registers: Chip8ProcessorRegisters,
    stack: ArrayVec<u16, 16>,
    execution_state: ExecutionState,
}

impl Default for ProcessorState {
    fn default() -> Self {
        Self {
            stack: ArrayVec::default(),
            registers: Chip8ProcessorRegisters::default(),
            execution_state: ExecutionState::Normal,
        }
    }
}

pub struct Chip8Processor {
    processor_state: Arc<RwLock<ProcessorState>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chip8ProcessorSnapshot {
    registers: Chip8ProcessorRegisters,
    stack: ArrayVec<u16, 16>,
    execution_state: ExecutionState,
}

impl Component for Chip8Processor {
    fn reset(&self) {
        let mut state = self.processor_state.write().unwrap();

        *state = ProcessorState::default();
    }
}

impl FromConfig for Chip8Processor {
    type Config = Chip8ProcessorConfig;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
    ) where
        Self: Sized,
    {
        let frequency = config.frequency;
        let state = Arc::new(RwLock::new(ProcessorState::default()));

        // Optionally initialize the jit engine
        #[cfg(jit)]
        let jit =
            if essentials.environment().processor_execution_mode == ProcessorExecutionMode::Jit {
                Some(
                    multiemu_machine::processor::jit::InstructionJitExecutor::new(
                        jit::Chip8InstructionTranslator::new(config.kind),
                    ),
                )
            } else {
                None
            };

        component_builder
            .insert_task(
                frequency,
                Chip8ProcessorTask {
                    state: state.clone(),
                    instruction_decoder: Chip8InstructionDecoder,
                    #[cfg(jit)]
                    jit,
                    virtual_gamepad: Option::default(),
                    display_component: essentials
                        .component_store()
                        .get(config.display_component_id)
                        .unwrap(),
                    timer_component: essentials
                        .component_store()
                        .get(config.timer_component_id)
                        .unwrap(),
                    audio_component: essentials
                        .component_store()
                        .get(config.audio_component_id)
                        .unwrap(),
                    essentials,
                    config,
                },
            )
            .insert_gamepads(
                [(
                    CHIP8_KEYPAD_GAMEPAD_TYPE,
                    VirtualGamepadMetadata {
                        present_inputs: present_inputs(),
                        default_bindings: default_bindings(),
                    },
                )],
                [CHIP8_KEYPAD_GAMEPAD_TYPE],
            )
            .build_global(Self {
                processor_state: state.clone(),
            });
    }
}
