use std::{
    borrow::Cow,
    io::{Read, Write},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use arrayvec::ArrayVec;
use input::{Chip8KeyCode, default_bindings, present_inputs};
use instruction::Register;
use multiemu_runtime::{
    component::{
        Component, ComponentConfig, ComponentVersion, SynchronizationContext, TypedComponentHandle,
    },
    input::{VirtualGamepad, VirtualGamepadMetadata},
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    memory::{AddressSpace, AddressSpaceId},
    path::MultiemuPath,
    platform::Platform,
    scheduler::{Frequency, Period},
};
use serde::{Deserialize, Serialize};

use super::Chip8Mode;
use crate::{
    audio::Chip8Audio,
    display::{Chip8Display, SupportedGraphicsApiChip8Display},
    processor::decoder::decode_instruction,
    timer::Chip8Timer,
};

pub mod decoder;
mod input;
mod instruction;
mod interpret;
// #[cfg(jit)]
// mod jit;

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

// This is extremely complex because the chip8 cpu has a lot of non cpu
// machinery

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
pub struct Chip8Processor<G: SupportedGraphicsApiChip8Display> {
    state: ProcessorState,
    /// Keypad virtual gamepad
    virtual_gamepad: Arc<VirtualGamepad>,
    /// Essential stuff the runtime provides
    cpu_address_space: Arc<AddressSpace>,
    // What chip8 mode we are currently in
    mode: Arc<Mutex<Chip8Mode>>,
    display: TypedComponentHandle<Chip8Display<G>>,
    audio: TypedComponentHandle<Chip8Audio>,
    timer: TypedComponentHandle<Chip8Timer>,
    config: Chip8ProcessorConfig<G>,
    timestamp: Period,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chip8ProcessorSnapshot {
    registers: Chip8ProcessorRegisters,
    stack: ArrayVec<u16, 16>,
    execution_state: ExecutionState,
}
impl<G: SupportedGraphicsApiChip8Display> Chip8Processor<G> {
    pub fn frequency(&self) -> Frequency {
        self.config.frequency
    }
}

impl<G: SupportedGraphicsApiChip8Display> Component for Chip8Processor<G> {
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

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        while context.allocate_period(self.config.frequency.recip()) {
            self.timestamp = context.now();

            'main: {
                match &self.state.execution_state {
                    ExecutionState::Normal => {
                        let mut instruction = [0; 2];

                        self.cpu_address_space
                            .read(
                                self.state.registers.program as usize,
                                self.timestamp,
                                None,
                                &mut instruction,
                            )
                            .unwrap();

                        let instruction =
                            decode_instruction(instruction).expect("Failed to decode instruction");

                        self.state.registers.program = self.state.registers.program.wrapping_add(2);

                        self.interpret_instruction(instruction);
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
                            self.state.execution_state = ExecutionState::AwaitingKeyRelease {
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
                                self.state.registers.work_registers[register as usize] = key_code.0;
                                self.state.execution_state = ExecutionState::Normal;
                                break 'main;
                            }
                        }
                    }
                    ExecutionState::AwaitingVsync => {
                        let vsync_occured = self
                            .display
                            .interact(self.timestamp, |component| component.vsync_occurred);

                        if vsync_occured {
                            self.state.execution_state = ExecutionState::Normal;
                            break 'main;
                        }
                    }
                    ExecutionState::Halted => {
                        // Do nothing
                    }
                }
            }
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= self.config.frequency.recip()
    }
}

#[derive(Debug)]
pub struct Chip8ProcessorConfig<G: SupportedGraphicsApiChip8Display> {
    pub cpu_address_space: AddressSpaceId,
    pub display: MultiemuPath,
    pub audio: MultiemuPath,
    pub timer: MultiemuPath,
    pub frequency: Frequency,
    pub force_mode: Option<Chip8Mode>,
    pub always_shr_in_place: bool,
    pub _phantom: PhantomData<fn() -> G>,
}

impl<P: Platform<GraphicsApi: SupportedGraphicsApiChip8Display>> ComponentConfig<P>
    for Chip8ProcessorConfig<P::GraphicsApi>
{
    type Component = Chip8Processor<P::GraphicsApi>;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let address_space = component_builder
            .get_address_space(self.cpu_address_space)
            .clone();

        let mode = Arc::new(Mutex::new(self.force_mode.unwrap_or(Chip8Mode::Chip8)));
        let state = ProcessorState::default();

        let virtual_gamepad = VirtualGamepad::new(Cow::Owned(VirtualGamepadMetadata {
            present_inputs: present_inputs(),
            default_real2virtual_mappings: default_bindings(),
        }));

        let (component_builder, _) = component_builder
            .set_scheduler_participation(SchedulerParticipation::SchedulerDriven)
            .insert_gamepad("chip8-keypad", virtual_gamepad.clone());

        let display = component_builder.typed_handle(&self.display).unwrap();
        let audio = component_builder.typed_handle(&self.audio).unwrap();
        let timer = component_builder.typed_handle(&self.timer).unwrap();

        Ok(Chip8Processor {
            state,
            virtual_gamepad: virtual_gamepad.clone(),
            cpu_address_space: address_space,
            mode,
            display,
            audio,
            timer,
            config: self,
            timestamp: Period::default(),
        })
    }
}
