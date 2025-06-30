use arrayvec::ArrayVec;
use bitvec::{order::Msb0, view::BitView};
use decoder::Mos6502InstructionDecoder;
use instruction::Mos6502InstructionSet;
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, ComponentRef},
    memory::AddressSpaceHandle,
    platform::Platform,
};
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use task::Mos6502Task;

mod decoder;
mod instruction;
mod interpret;
mod task;
#[cfg(test)]
mod tests;

const RESET_VECTOR: usize = 0xfffc;
const PAGE_SIZE: usize = 256;

// Addressing modes vs load steps
//
// We will start with the program pointer on the address bus
//
// AddressingMode::Mos6502(Mos6502AddressingMode::Immediate)
//      LoadStep::Opcode
//
// AddressingMode::Mos6502(Mos6502AddressingMode::Absolute)
//      LoadStep::Opcode
//      LoadStep::Data (low byte)
//      LoadStep::Data (high byte)
//      LoadStep::LatchToBus
//
// AddressingMode::Mos6502(Mos6502AddressingMode::AbsoluteIndirect)
//      LoadStep::Opcode
//      LoadStep::Data (low byte of indirect address)
//      LoadStep::Data (high byte of indirect address)
//      LoadStep::LatchToBus
//      LoadStep::Data (fetch low byte of pointer as immediate)
//      LoadStep::Data (fetch high byte of pointer as immediate)
//      LoadStep::LatchToBus
//
// AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute)
//      LoadStep::Opcode
//      LoadStep::Data (low byte)
//      LoadStep::Data (high byte)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add X) <- this might be done in parallel depending on the instruction
//
// AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute)
//      LoadStep::Opcode
//      LoadStep::Data (low byte)
//      LoadStep::Data (high byte)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add Y) <- this might be done in parallel depending on the instruction
//
// AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPage)
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//
// AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage)
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add X) <- this might be done in parallel depending on the instruction
//
// AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedZeroPage)
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add Y) <- this might be done in parallel depending on the instruction
//
// AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPageIndirect)
//      LoadStep::Opcode
//      LoadStep::Data (zero page offset)
//      LoadStep::LatchToBus
//      LoadStep::Offset (add X) <- this might be done in parallel depending on the instruction
//      LoadStep::Data (fetch low byte of pointer as immediate)
//      LoadStep::Data (fetch high byte of pointer as immediate)
//      LoadStep::LatchToBus
//
// AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPageIndirectYIndexed)
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
    /// Processor is awaiting a interrupt
    Wait,
    /// Fetch and decode instruction
    FetchAndDecode,
    /// Loads data required for the instruction
    PreInterpret {
        instruction: Mos6502InstructionSet,
        latch: ArrayVec<u8, 2>,
        queue: VecDeque<LoadStep>,
    },
    /// Execute this instruction
    Interpret { instruction: Mos6502InstructionSet },
    /// Stores data for the instruction
    PostInterpret { queue: VecDeque<StoreStep> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Mos6502Kind {
    /// Standard
    Mos6502,
    /// Slimmed down atari 2600 version
    Mos6507,
    /// NES version
    Ricoh2A0x,
    // Upgraded version
    Wdc65C02,
}

impl Mos6502Kind {
    pub fn original_instruction_set(&self) -> bool {
        matches!(self, Self::Mos6502 | Self::Mos6507 | Self::Ricoh2A0x)
    }
}

impl Mos6502Kind {
    pub fn supports_decimal(&self) -> bool {
        !matches!(self, Mos6502Kind::Ricoh2A0x)
    }
}

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
/// We don't store this in memory bitpacked for performance reasons
pub struct FlagRegister {
    negative: bool,
    overflow: bool,
    undocumented: bool,
    break_: bool,
    decimal: bool,
    interrupt_disable: bool,
    zero: bool,
    carry: bool,
}

// Do it manually because deku is slow
impl FlagRegister {
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0;
        let bits = byte.view_bits_mut::<Msb0>();

        bits.set(0, self.negative);
        bits.set(1, self.overflow);
        bits.set(2, self.undocumented);
        bits.set(3, self.break_);
        bits.set(4, self.decimal);
        bits.set(5, self.interrupt_disable);
        bits.set(6, self.zero);
        bits.set(7, self.carry);

        byte
    }

    pub fn from_byte(byte: u8) -> Self {
        let bits = byte.view_bits::<Msb0>();

        Self {
            negative: bits[0],
            overflow: bits[1],
            undocumented: bits[2],
            break_: bits[3],
            decimal: bits[4],
            interrupt_disable: bits[5],
            zero: bits[6],
            carry: bits[7],
        }
    }
}

#[derive(Debug)]
pub struct Mos6502Config {
    pub frequency: Ratio<u32>,
    pub assigned_address_space: AddressSpaceHandle,
    pub kind: Mos6502Kind,
    pub broken_ror: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    rdy: AtomicBool,
    config: Mos6502Config,
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
    fn on_reset(&self) {
        self.set_rdy(true);
        *self.state.lock().unwrap() = ProcessorState::default();
    }
}

impl<P: Platform> ComponentConfig<P> for Mos6502Config {
    type Component = Mos6502;

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) {
        let rdy = AtomicBool::new(true);
        let memory_translation_table = component_builder
            .essentials()
            .memory_translation_table
            .clone();

        component_builder
            .insert_task(
                self.frequency,
                Mos6502Task {
                    memory_translation_table,
                    instruction_decoder: Mos6502InstructionDecoder::new(self.kind),
                    component: component_ref,
                },
            )
            .build_global(Mos6502 {
                state: Mutex::default(),
                rdy,
                config: self,
            })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    rdy: bool,
    state: ProcessorState,
}
