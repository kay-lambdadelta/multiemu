use decoder::M6502InstructionDecoder;
use enumflags2::{BitFlags, bitflags};
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::AddressSpaceId,
};
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use task::M6502Task;

pub mod decoder;
pub mod instruction;
pub mod interpret;
pub mod task;

pub const RESET_VECTOR: usize = 0xfffc;
const PAGE_SIZE: usize = 256;

#[cfg(test)]
pub mod test;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExecutionMode {
    Startup,
    Reset,
    Normal,
    Jammed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum M6502Kind {
    /// Standard
    M6502,
    /// Slimmed down atari 2600 version
    M6507,
    /// NES version
    R2A0x,
}

impl M6502Kind {
    pub fn supports_decimal(&self) -> bool {
        !matches!(self, M6502Kind::R2A0x)
    }
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FlagRegister {
    /// Set when bit 7 is set on various math operations
    Negative = 0b1000_0000,
    /// Set when a math operation involves an overflow
    Overflow = 0b0100_0000,
    /// This flag is usually 1, it doesn't mean anything
    __Unused = 0b0010_0000,
    /// Flag to inform software the reason behind some behaviors
    Break = 0b0001_0000,
    /// Decimal math mode, it enables bcd operations on a lot of math instructions and introduces some bugs
    Decimal = 0b0000_1000,
    /// Interrupt disable
    InterruptDisable = 0b0000_0100,
    /// Set when the result of a math operation is 0
    Zero = 0b0000_0010,
    Carry = 0b0000_0001,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[cfg_attr(jit, repr(C))]
pub struct M6502Registers {
    pub stack: u8,
    pub accumulator: u8,
    pub index_registers: [u8; 2],
    pub flags: BitFlags<FlagRegister>,
    pub program: u16,
}

impl Default for M6502Registers {
    fn default() -> Self {
        Self {
            stack: 0xff,
            accumulator: 0,
            index_registers: [0; 2],
            flags: BitFlags::empty(),
            program: 0,
        }
    }
}

#[derive(Debug)]
pub struct M6502Config {
    pub frequency: Ratio<u64>,
    pub assigned_address_space: AddressSpaceId,
    pub kind: M6502Kind,
    pub initial_state: M6502Registers,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[cfg_attr(jit, repr(C))]
struct ProcessorState {
    registers: M6502Registers,
    cycle_counter: u8,
    execution_mode: ExecutionMode,
}

pub struct M6502 {
    state: Mutex<ProcessorState>,
}

impl Component for M6502 {
    fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.execution_mode = ExecutionMode::Reset;
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct M6502Quirks {
    pub broken_ror: bool,
}

impl FromConfig for M6502 {
    type Config = M6502Config;
    type Quirks = M6502Quirks;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let config = Arc::new(config);

        component_builder
            .insert_task(
                config.frequency,
                M6502Task {
                    essentials: essentials.clone(),
                    instruction_decoder: M6502InstructionDecoder,
                    config: config.clone(),
                },
            )
            .build(Self {
                state: Mutex::new(ProcessorState {
                    registers: M6502Registers::default(),
                    cycle_counter: 0,
                    execution_mode: ExecutionMode::Startup,
                }),
            });
    }
}
