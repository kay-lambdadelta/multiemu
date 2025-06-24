use super::Chip8Mode;
use crate::{
    Chip8InstructionDecoder,
    audio::Chip8Audio,
    display::{Chip8Display, SupportedGraphicsApiChip8Display},
    timer::Chip8Timer,
};
use arrayvec::ArrayVec;
use input::{CHIP8_KEYPAD_GAMEPAD_TYPE, Chip8KeyCode, default_bindings, present_inputs};
use instruction::Register;
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, ComponentId, ComponentRef},
    input::{VirtualGamepad, VirtualGamepadMetadata},
    memory::AddressSpaceHandle,
    platform::Platform,
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
    fn on_reset(&self) {
        let mut state = self.state.lock().unwrap();

        *state = ProcessorState::default();
    }
}

#[derive(Debug)]
pub struct Chip8ProcessorConfig<G: SupportedGraphicsApiChip8Display> {
    pub cpu_address_space: AddressSpaceHandle,
    pub display: ComponentRef<Chip8Display<G>>,
    pub audio: ComponentRef<Chip8Audio>,
    pub timer: ComponentRef<Chip8Timer>,
    pub frequency: Ratio<u32>,
    pub force_mode: Option<Chip8Mode>,
    pub always_shr_in_place: bool,
}

impl<P: Platform<GraphicsApi: SupportedGraphicsApiChip8Display>> ComponentConfig<P>
    for Chip8ProcessorConfig<P::GraphicsApi>
{
    type Component = Chip8Processor;

    fn build_dependencies(&self) -> impl IntoIterator<Item = ComponentId> {
        [self.display.id(), self.audio.id(), self.timer.id()]
    }

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) {
        let memory_translation_table = component_builder
            .essentials()
            .memory_translation_table
            .clone();
        let mode = Arc::new(Mutex::new(self.force_mode.unwrap_or(Chip8Mode::Chip8)));
        let state = Mutex::new(ProcessorState::default());

        let virtual_gamepad = VirtualGamepad::new(
            CHIP8_KEYPAD_GAMEPAD_TYPE,
            VirtualGamepadMetadata {
                present_inputs: present_inputs(),
                default_bindings: default_bindings(),
            },
        );

        component_builder
            .insert_gamepad(virtual_gamepad.clone())
            .insert_task(
                self.frequency,
                Chip8ProcessorTask {
                    instruction_decoder: Chip8InstructionDecoder,
                    virtual_gamepad,
                    memory_translation_table,
                    mode,
                    config: self,
                    component: component_ref,
                },
            )
            .build_global(Chip8Processor { state })
    }
}
