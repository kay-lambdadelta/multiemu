use crate::{Chip8InstructionDecoder, display::Chip8Display};

use super::Chip8Kind;
use arrayvec::ArrayVec;
use crossbeam::atomic::AtomicCell;
use input::{CHIP8_KEYPAD_GAMEPAD_TYPE, Chip8KeyCode, default_bindings, present_inputs};
use instruction::Register;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    input::virtual_gamepad::{VirtualGamepad, VirtualGamepadMetadata},
    memory::AddressSpaceHandle,
};
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use task::Chip8ProcessorTask;

pub mod decoder;
mod input;
mod instruction;
mod interpret;
// #[cfg(jit)]
// mod jit;
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
    AwaitingVsync,
}

// This is extremely complex because the chip8 cpu has a lot of non cpu machinery

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Chip8ProcessorQuirks {
    pub frequency: Ratio<u32>,
    pub force_mode: Option<Chip8Kind>,
    pub always_shr_in_place: bool,
}

impl Default for Chip8ProcessorQuirks {
    fn default() -> Self {
        Self {
            frequency: Ratio::from_integer(700),
            force_mode: None,
            always_shr_in_place: false,
        }
    }
}

#[derive(Debug)]
// #[cfg_attr(jit, repr(C))]
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
    state: Mutex<ProcessorState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chip8ProcessorSnapshot {
    registers: Chip8ProcessorRegisters,
    stack: ArrayVec<u16, 16>,
    execution_state: ExecutionState,
}

impl Component for Chip8Processor {
    fn reset(&self) {
        let mut state = self.state.lock().unwrap();

        *state = ProcessorState::default();
    }
}

#[derive(Debug)]
pub struct Chip8ProcessorConfig {
    pub cpu_address_space: AddressSpaceHandle,
}

impl FromConfig for Chip8Processor {
    type Config = Chip8ProcessorConfig;
    type Quirks = Chip8ProcessorQuirks;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        quirks: Self::Quirks,
    ) where
        Self: Sized,
    {
        let quirks = Arc::new(quirks);
        let mode = Arc::new(AtomicCell::new(
            quirks.force_mode.unwrap_or(Chip8Kind::Chip8),
        ));
        let frequency = quirks.frequency;
        let state = Mutex::new(ProcessorState::default());

        let virtual_gamepad = VirtualGamepad::new(
            CHIP8_KEYPAD_GAMEPAD_TYPE,
            VirtualGamepadMetadata {
                present_inputs: present_inputs(),
                default_bindings: default_bindings(),
            },
        );

        let display_component = essentials.component_store.get("display").unwrap();

        let mut vsync = None;
        display_component
            .interact(|component: &Chip8Display| {
                vsync = Some(component.vsync_occurred.clone());
            })
            .unwrap();

        component_builder
            .insert_gamepads([virtual_gamepad.clone()])
            .insert_task(
                frequency,
                Chip8ProcessorTask {
                    instruction_decoder: Chip8InstructionDecoder,
                    virtual_gamepad,
                    vsync_occurred: vsync.unwrap(),
                    display_component,
                    timer_component: essentials.component_store.get("timer").unwrap(),
                    audio_component: essentials.component_store.get("audio").unwrap(),
                    essentials,
                    quirks,
                    mode,
                    config,
                },
            )
            .build_global(Self { state });
    }
}
