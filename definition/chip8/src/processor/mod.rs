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
    component::{BuildError, Component, ComponentConfig, ComponentRef, ComponentVersion},
    input::{VirtualGamepad, VirtualGamepadMetadata},
    memory::AddressSpaceHandle,
    platform::Platform,
};
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};
use task::CpuDriver;

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
    fn reset(&self) {
        let mut state = self.state.lock().unwrap();

        *state = ProcessorState::default();
    }

    fn load_snapshot(
        &self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);

        let snapshot: Chip8ProcessorSnapshot =
            bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;
        let mut state_guard = self.state.lock().unwrap();

        state_guard.registers = snapshot.registers;
        state_guard.stack = snapshot.stack;
        state_guard.execution_state = snapshot.execution_state;
        Ok(())
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let state_guard = self.state.lock().unwrap();
        let snapshot = Chip8ProcessorSnapshot {
            registers: state_guard.registers.clone(),
            stack: state_guard.stack.clone(),
            execution_state: state_guard.execution_state.clone(),
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

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let memory_access_table = component_builder.memory_access_table();
        let mode = Arc::new(Mutex::new(self.force_mode.unwrap_or(Chip8Mode::Chip8)));
        let state = Mutex::new(ProcessorState::default());
        let component = component_builder.component_ref();

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
                "driver",
                CpuDriver {
                    instruction_decoder: Chip8InstructionDecoder,
                    virtual_gamepad,
                    memory_access_table,
                    mode,
                    config: self,
                    component,
                },
            )
            .build_global(|_| Chip8Processor { state });

        Ok(())
    }
}
