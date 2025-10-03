use arrayvec::ArrayVec;
use bitvec::{order::Msb0, view::BitView};
use decoder::Mos6502InstructionDecoder;
use instruction::Mos6502InstructionSet;
use multiemu::{
    component::{BuildError, Component, ComponentConfig, ComponentVersion},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
};
use num::rational::Ratio;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use task::CpuDriver;

mod decoder;
mod instruction;
mod interpret;
mod task;
#[cfg(test)]
mod tests;

const RESET_VECTOR: u16 = 0xfffc;
const IRQ_VECTOR: u16 = 0xfffe;
const NMI_VECTOR: u16 = 0xfffa;
const PAGE_SIZE: usize = 256;

// NOTE: This is based upon an old design of this cpu emulator but I'm keeping it until its fully implemented
//
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
//      LoadStep::Offset (add Y)  <- this might be done in parallel depending on the instructions

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressBusModification {
    X,
    Y,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStep {
    /// Resets the processor
    Reset,
    /// Processor is jammed forever and cannot do anything until a reset
    Jammed,
    /// Processor is awaiting a interrupt
    Wait,
    /// Fetch and decode instruction
    FetchAndDecode,
    /// Loading data, pushing it to the latch, from the address bus pointer
    LoadData,
    /// Same as before but it isn't referencing memory
    LoadDataFromConstant(u8),
    /// Putting data into memory, from the address bus pointer
    StoreData(u8),
    /// Processor is storing an item on the stack
    PushStack(u8),
    /// Moving item from the latch to the address bus
    LatchToAddressBus,
    /// Moving item from the latch to the program pointer
    LatchToProgramPointer,
    /// Copying address bus to program pointer
    AddressBusToProgramPointer,
    /// Adding this value to the program pointer
    ModifyProgramPointer(i8),
    /// Adding this value to the address bus
    ModifyAddressBus(AddressBusModification),
    /// Execute this instruction
    Interpret { instruction: Mos6502InstructionSet },
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

    pub fn supports_decimal(&self) -> bool {
        !matches!(self, Mos6502Kind::Ricoh2A0x)
    }

    pub fn supports_interrupts(&self) -> bool {
        !matches!(self, Mos6502Kind::Mos6507)
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
    pub assigned_address_space: AddressSpaceId,
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
    /// What the processor is currently doing right now
    pub execution_queue: VecDeque<ExecutionStep>,
    /// Address bus
    pub address_bus: u16,
    /// Imaginary processor latch
    pub latch: ArrayVec<u8, 2>,
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
            execution_queue: VecDeque::from_iter([ExecutionStep::Reset]),
            address_bus: 0x0000,
            latch: ArrayVec::default(),
        }
    }
}

#[derive(Debug)]
pub struct Mos6502 {
    state: ProcessorState,
    rdy: Arc<RdyFlag>,
    irq: Arc<IrqFlag>,
    nmi: Arc<NmiFlag>,
    config: Mos6502Config,
}

impl Mos6502 {
    pub fn rdy(&self) -> Arc<RdyFlag> {
        self.rdy.clone()
    }

    pub fn irq(&self) -> Arc<IrqFlag> {
        self.irq.clone()
    }

    pub fn nmi(&self) -> Arc<NmiFlag> {
        self.nmi.clone()
    }
}

impl Component for Mos6502 {
    fn reset(&mut self) {
        self.rdy.store(true);
        self.state = ProcessorState::default();
    }

    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        bincode::serde::encode_into_std_write(
            &Snapshot {
                rdy: self.rdy.load(),
                state: self.state.clone(),
            },
            &mut writer,
            bincode::config::standard(),
        )?;

        Ok(())
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match version {
            0 => {
                let snapshot: Snapshot =
                    bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;

                self.rdy.store(snapshot.rdy);
                self.state = snapshot.state;

                Ok(())
            }
            other => Err(format!("Unsupported snapshot version: {}", other).into()),
        }
    }
}

impl<P: Platform> ComponentConfig<P> for Mos6502Config {
    type Component = Mos6502;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let memory_access_table = component_builder.memory_access_table();

        component_builder
            .insert_task_mut(
                "driver",
                self.frequency,
                CpuDriver {
                    memory_access_table,
                    instruction_decoder: Mos6502InstructionDecoder::new(self.kind),
                },
            )
            .build(Mos6502 {
                rdy: Arc::new(RdyFlag::new()),
                irq: Arc::new(IrqFlag::new()),
                nmi: Arc::new(NmiFlag::new()),
                state: ProcessorState::default(),
                config: self,
            });

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    rdy: bool,
    state: ProcessorState,
}

#[derive(Debug)]
pub struct RdyFlag(AtomicBool);

impl RdyFlag {
    pub fn new() -> Self {
        Self(AtomicBool::new(true))
    }

    pub fn store(&self, rdy: bool) {
        if rdy {
            tracing::debug!("RDY went high, resuming execution");
        } else {
            tracing::debug!("RDY went low, pausing execution");
        }

        self.0.store(rdy, Ordering::Release);
    }

    pub fn load(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
pub struct IrqFlag(AtomicBool);

impl IrqFlag {
    pub fn new() -> Self {
        Self(AtomicBool::new(true))
    }

    pub fn store(&self, irq: bool) {
        self.0.store(irq, Ordering::Release);
    }

    pub fn interrupt_required(&self) -> bool {
        !self.0.load(Ordering::Acquire)
    }
}

/// NMI is falling edge
#[derive(Debug)]
pub struct NmiFlag(AtomicBool);

impl NmiFlag {
    pub fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    pub fn store(&self, nmi: bool) {
        if nmi {
            self.0.store(true, Ordering::Release);
        }
    }

    pub fn interrupt_required(&self) -> bool {
        self.0.swap(false, Ordering::AcqRel)
    }
}
