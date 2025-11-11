use super::Chip8Mode;
use crate::{Chip8InstructionDecoder, display::SupportedGraphicsApiChip8Display};
use arrayvec::ArrayVec;
use input::{Chip8KeyCode, default_bindings, present_inputs};
use instruction::Register;
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentPath, ComponentVersion},
    input::{VirtualGamepad, VirtualGamepadMetadata},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
    scheduler::TaskType,
};
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    io::{Read, Write},
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use task::Driver;

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
    state: ProcessorState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chip8ProcessorSnapshot {
    registers: Chip8ProcessorRegisters,
    stack: ArrayVec<u16, 16>,
    execution_state: ExecutionState,
}

impl Component for Chip8Processor {
    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);

        let snapshot: Chip8ProcessorSnapshot =
            bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;

        self.state.registers = snapshot.registers;
        self.state.stack = snapshot.stack;
        self.state.execution_state = snapshot.execution_state;

        Ok(())
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let snapshot = Chip8ProcessorSnapshot {
            registers: self.state.registers.clone(),
            stack: self.state.stack.clone(),
            execution_state: self.state.execution_state.clone(),
        };

        bincode::serde::encode_into_std_write(&snapshot, &mut writer, bincode::config::standard())?;
        Ok(())
    }

    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }
}

#[derive(Debug)]
pub struct Chip8ProcessorConfig<G: SupportedGraphicsApiChip8Display> {
    pub cpu_address_space: AddressSpaceId,
    pub display: ComponentPath,
    pub audio: ComponentPath,
    pub timer: ComponentPath,
    pub frequency: Ratio<u32>,
    pub force_mode: Option<Chip8Mode>,
    pub always_shr_in_place: bool,
    pub _phantom: PhantomData<fn() -> G>,
}

impl<P: Platform<GraphicsApi: SupportedGraphicsApiChip8Display>> ComponentConfig<P>
    for Chip8ProcessorConfig<P::GraphicsApi>
{
    type Component = Chip8Processor;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let address_space = component_builder.address_space(self.cpu_address_space);
        let mode = Arc::new(Mutex::new(self.force_mode.unwrap_or(Chip8Mode::Chip8)));
        let state = ProcessorState::default();
        let registry = component_builder.registry();

        let virtual_gamepad = VirtualGamepad::new(Cow::Owned(VirtualGamepadMetadata {
            present_inputs: present_inputs(),
            default_real2virtual_mappings: default_bindings(),
        }));
        let frequency = self.frequency;

        let driver = Driver {
            instruction_decoder: Chip8InstructionDecoder,
            virtual_gamepad: virtual_gamepad.clone(),
            cpu_address_space: address_space,
            mode,
            display: registry.get(&self.display).unwrap(),
            audio: registry.get(&self.audio).unwrap(),
            timer: registry.get(&self.timer).unwrap(),
            config: self,
        };

        component_builder
            .insert_gamepad("chip8-keypad", virtual_gamepad)
            .0
            .insert_task("driver", frequency, TaskType::Direct, driver);

        Ok(Chip8Processor { state })
    }
}
