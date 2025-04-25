use arrayvec::ArrayVec;
use decoder::M6502InstructionDecoder;
use enumflags2::{BitFlags, bitflags};
use instruction::M6502InstructionSet;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::AddressSpaceId,
};
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use task::M6502Task;

mod decoder;
mod instruction;
mod interpret;
mod task;

pub const RESET_VECTOR: usize = 0xfffc;
const PAGE_SIZE: usize = 256;

// Addressing modes vs load steps
//
// We will start with the program pointer on the address bus
//
// AddressingMode::Immediate
//      LoadStep::Opcode
//
// AddressingMode::Absolute
//      LoadStep::Opcode
//      LoadStep::Data (low byte)
//      LoadStep::Data (high byte)
//      LoadStep::LatchToBus
//
// AddressingMode::AbsoluteIndirect
//      LoadStep::Opcode
//      LoadStep::Data (low byte of indirect address)
//      LoadStep::Data (high byte of indirect address)
//      LoadStep::LatchToBus
//      LoadStep::Data (fetch low byte of pointer as immediate)
//      LoadStep::Data (fetch high byte of pointer as immediate)
//      LoadStep::LatchToBus
//
// AddressingMode::XIndexedAbsolute
//      LoadStep::Opcode
//      LoadStep::Data (low byte)
//      LoadStep::Data (high byte)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add X) <- this might be done in parallel depending on the instruction
//
// AddressingMode::YIndexedAbsolute
//      LoadStep::Opcode
//      LoadStep::Data (low byte)
//      LoadStep::Data (high byte)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add Y) <- this might be done in parallel depending on the instruction
//
// AddressingMode::ZeroPage
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//
// AddressingMode::XIndexedZeroPage
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add X) <- this might be done in parallel depending on the instruction
//
// AddressingMode::YIndexedZeroPage
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add Y) <- this might be done in parallel depending on the instruction
//
// AddressingMode::XIndexedZeroPageIndirect
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add X) <- this might be done in parallel depending on the instruction
//      LoadStep::Data (fetch low byte of pointer as immediate)
//      LoadStep::Data (fetch high byte of pointer as immediate)
//      LoadStep::LatchToBus
//
// AddressingMode::ZeroPageIndirectYIndexed
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//      LoadStep::Data (fetch low byte of pointer as immediate)
//      LoadStep::Data (fetch high byte of pointer as immediate)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add Y)  <- this might be done in parallel depending on the instruction

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadStep {
    /// The CPU is fetching a byte of data so it can continue to the next operation
    Data,
    /// The CPU is moving the contents of the temporary latch to the bus. This is a pseudo step so it doesnt consume a cycle
    LatchToBus,
    /// The CPU is computing an offset to the data its loading
    Offset { offset: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreStep {
    // The CPU is putting the contents of this latch into memory
    Data {
        /// The CPU is storing a byte of data
        value: u8,
    },
    PushStack {
        /// The CPU is pushing a byte to the stack
        data: u8,
    },
    /// The CPU is putting the contents of the address bus onto the program counter ("jump")
    BusToProgram,
    /// The CPU is relatively modifying the program counter
    AddToProgram { value: i8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Resets the processor
    Reset,
    /// Processor is jammed forever and cannot do anything until a reset
    Jammed,
    /// Fetch and decode instruction
    Fetch,
    /// Fetches and decodes the next instruction
    Load {
        instruction: M6502InstructionSet,
        latch: ArrayVec<u8, 2>,
        queue: VecDeque<LoadStep>,
    },
    /// Execute this instruction
    Execute {
        instruction: M6502InstructionSet,
    },
    Store {
        queue: VecDeque<StoreStep>,
    },
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

#[derive(Debug)]
pub struct M6502Config {
    pub frequency: Ratio<u32>,
    pub assigned_address_space: AddressSpaceId,
    pub kind: M6502Kind,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(jit, repr(C))]
struct ProcessorState {
    /// Accumulator
    pub a: u8,
    /// X index register
    pub x: u8,
    /// Y index register
    pub y: u8,
    /// Flag register
    pub flags: BitFlags<FlagRegister>,
    /// Stack pointer
    pub stack: u8,
    /// Program pointer
    pub program: u16,
    pub execution_mode: Option<ExecutionMode>,
    /// Address bus
    pub address_bus: u16,
}

impl Default for ProcessorState {
    fn default() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            flags: BitFlags::empty(),
            stack: 0xff,
            // Will be set later
            program: 0x0000,
            execution_mode: Some(ExecutionMode::Reset),
            address_bus: 0x0000,
        }
    }
}

#[derive(Debug)]
pub struct M6502 {
    state: Mutex<ProcessorState>,
    rdy: Arc<AtomicBool>,
}

impl M6502 {
    pub fn set_rdy(&self, rdy: bool) {
        if rdy {
            tracing::debug!("RDY went high, resuming execution");
        } else {
            tracing::debug!("RDY went low, pausing execution");
        }

        self.rdy.store(rdy, Ordering::Relaxed);
    }
}

impl Component for M6502 {
    fn reset(&self) {
        todo!()
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
        let rdy = Arc::new(AtomicBool::new(true));

        component_builder
            .insert_task(
                config.frequency,
                M6502Task {
                    essentials: essentials.clone(),
                    instruction_decoder: M6502InstructionDecoder,
                    config: config.clone(),
                },
            )
            .build_global(Self {
                state: Mutex::default(),
                rdy,
            });
    }
}
