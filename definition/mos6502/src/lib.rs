use arrayvec::ArrayVec;
use decoder::Mos6502InstructionDecoder;
use deku::{DekuRead, DekuWrite};
use instruction::Mos6502InstructionSet;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::memory_translation_table::address_space::AddressSpaceHandle,
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
use task::Mos6502Task;

mod decoder;
mod instruction;
mod interpret;
mod task;

const RESET_VECTOR: usize = 0xfffc;
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
    /// Loads data required for the instruction
    Load {
        instruction: Mos6502InstructionSet,
        latch: ArrayVec<u8, 2>,
        queue: VecDeque<LoadStep>,
    },
    /// Execute this instruction
    Execute { instruction: Mos6502InstructionSet },
    /// Stores data for the instruction
    Store { queue: VecDeque<StoreStep> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mos6502Kind {
    /// Standard
    Mos6502,
    /// Slimmed down atari 2600 version
    M6507,
    /// NES version
    R2A0x,
}

impl Mos6502Kind {
    pub fn supports_decimal(&self) -> bool {
        !matches!(self, Mos6502Kind::R2A0x)
    }
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug, Default, DekuRead, DekuWrite)]
pub struct FlagRegister {
    #[deku(bits = 1)]
    negative: bool,
    #[deku(bits = 1)]
    overflow: bool,
    #[deku(bits = 1)]
    undocumented: bool,
    #[deku(bits = 1)]
    break_: bool,
    #[deku(bits = 1)]
    decimal: bool,
    #[deku(bits = 1)]
    interrupt_disable: bool,
    #[deku(bits = 1)]
    zero: bool,
    #[deku(bits = 1)]
    carry: bool,
}

#[derive(Debug)]
pub struct Mos6502Config {
    pub frequency: Ratio<u32>,
    pub assigned_address_space: AddressSpaceHandle,
    pub kind: Mos6502Kind,
    pub broken_ror: bool,
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
    pub flags: FlagRegister,
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
            flags: FlagRegister::default(),
            stack: 0xff,
            // Will be set later
            program: 0x0000,
            execution_mode: Some(ExecutionMode::Reset),
            address_bus: 0x0000,
        }
    }
}

#[derive(Debug)]
pub struct Mos6502 {
    state: Mutex<ProcessorState>,
    rdy: Arc<AtomicBool>,
}

impl Mos6502 {
    pub fn set_rdy(&self, rdy: bool) {
        if rdy {
            tracing::debug!("RDY went high, resuming execution");
        } else {
            tracing::debug!("RDY went low, pausing execution");
        }

        self.rdy.store(rdy, Ordering::Relaxed);
    }
}

impl Component for Mos6502 {
    fn reset(&self) {
        self.set_rdy(true);
        *self.state.lock().unwrap() = ProcessorState::default();
    }
}

impl<R: RenderApi> ComponentConfig<R> for Mos6502Config {
    type Component = Mos6502;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        let config = Arc::new(self);
        let rdy = Arc::new(AtomicBool::new(true));
        let essentials = component_builder.essentials();

        component_builder
            .insert_task(
                config.frequency,
                Mos6502Task {
                    memory_translation_table: essentials.memory_translation_table.clone(),
                    instruction_decoder: Mos6502InstructionDecoder,
                    config: config.clone(),
                },
            )
            .build_global(Mos6502 {
                state: Mutex::default(),
                rdy,
            });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    rdy: bool,
    state: ProcessorState,
}
