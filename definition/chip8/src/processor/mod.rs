use super::Chip8Kind;
use crate::{
    Chip8InstructionDecoder, SupportedRenderApiChip8, audio::Chip8Audio, display::Chip8Display,
    timer::Chip8Timer,
};
use arrayvec::ArrayVec;
use crossbeam::atomic::AtomicCell;
use input::{CHIP8_KEYPAD_GAMEPAD_TYPE, Chip8KeyCode, default_bindings, present_inputs};
use instruction::Register;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, component_ref::ComponentRef},
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
    Halted,
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

#[derive(Debug)]
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
/// FIXME: This generic leakage is quite unacceptable, find a way to get rid of it while introducing minimal runtime casting
pub struct Chip8ProcessorConfig<R: SupportedRenderApiChip8> {
    pub cpu_address_space: AddressSpaceHandle,
    pub display: ComponentRef<Chip8Display<R>>,
    pub audio: ComponentRef<Chip8Audio>,
    pub timer: ComponentRef<Chip8Timer>,
    pub frequency: Ratio<u32>,
    pub force_mode: Option<Chip8Kind>,
    pub always_shr_in_place: bool,
}

impl<R: SupportedRenderApiChip8> ComponentConfig<R> for Chip8ProcessorConfig<R> {
    type Component = Chip8Processor;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>)
    where
        Self: Sized,
    {
        let essentials = component_builder.essentials();
        let mode = Arc::new(AtomicCell::new(self.force_mode.unwrap_or(Chip8Kind::Chip8)));
        let state = Mutex::new(ProcessorState::default());

        let virtual_gamepad = VirtualGamepad::new(
            CHIP8_KEYPAD_GAMEPAD_TYPE,
            VirtualGamepadMetadata {
                present_inputs: present_inputs(),
                default_bindings: default_bindings(),
            },
        );

        component_builder
            .insert_gamepads([virtual_gamepad.clone()])
            .insert_task(
                self.frequency,
                Chip8ProcessorTask {
                    instruction_decoder: Chip8InstructionDecoder,
                    virtual_gamepad,
                    memory_translation_table: essentials.memory_translation_table.clone(),
                    mode,
                    config: self,
                },
            )
            .build_global(Chip8Processor { state });
    }
}
