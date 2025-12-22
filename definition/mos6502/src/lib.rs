use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use arrayvec::ArrayVec;
use bitvec::{prelude::Lsb0, view::BitView};
use instruction::Mos6502InstructionSet;
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentVersion},
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    memory::{Address, AddressSpace, AddressSpaceCache, AddressSpaceId},
    platform::Platform,
    scheduler::{Frequency, Period, SynchronizationContext},
};
use serde::{Deserialize, Serialize};

use crate::{
    decoder::{
        InstructionGroup, decode_group1_space_instruction, decode_group2_space_instruction,
        decode_group3_space_instruction, decode_undocumented_space_instruction,
    },
    instruction::{AddressingMode, Mos6502AddressingMode, Wdc65C02AddressingMode},
    interpret::STACK_BASE_ADDRESS,
};

mod decoder;
mod instruction;
mod interpret;
#[cfg(test)]
mod tests;

pub const RESET_VECTOR: u16 = 0xfffc;
pub const IRQ_VECTOR: u16 = 0xfffe;
pub const NMI_VECTOR: u16 = 0xfffa;
const PAGE_SIZE: usize = 256;

// NOTE: This is based upon an old design of this cpu emulator but I'm keeping
// it until its fully implemented
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
//      LoadStep::Offset (add X) <- this might be done in parallel depending onthe instruction
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
    /// Handle the original mos 6502 absolute indirect errata
    LoadDataWithoutAdvancingPage,
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
    /// Make sure the address bus is within the zero page
    MaskAddressBusToZeroPage,
    /// Execute this instruction
    Interpret,
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
    #[inline]
    pub fn original_instruction_set(&self) -> bool {
        matches!(self, Self::Mos6502 | Self::Mos6507 | Self::Ricoh2A0x)
    }

    #[inline]
    pub fn supports_decimal(&self) -> bool {
        !matches!(self, Mos6502Kind::Ricoh2A0x)
    }

    #[inline]
    pub fn supports_interrupts(&self) -> bool {
        !matches!(self, Mos6502Kind::Mos6507)
    }

    #[inline]
    pub fn absolute_indirect_page_wrap_errata(&self) -> bool {
        matches!(self, Self::Mos6502 | Self::Mos6507 | Self::Ricoh2A0x)
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
        let bits = byte.view_bits_mut::<Lsb0>();

        bits.set(7, self.negative);
        bits.set(6, self.overflow);
        bits.set(5, self.undocumented);
        bits.set(4, self.break_);
        bits.set(3, self.decimal);
        bits.set(2, self.interrupt_disable);
        bits.set(1, self.zero);
        bits.set(0, self.carry);

        byte
    }

    pub fn from_byte(byte: u8) -> Self {
        let bits = byte.view_bits::<Lsb0>();

        Self {
            negative: bits[7],
            overflow: bits[6],
            undocumented: bits[5],
            break_: bits[4],
            decimal: bits[3],
            interrupt_disable: bits[2],
            zero: bits[1],
            carry: bits[0],
        }
    }
}

#[derive(Debug)]
pub struct Mos6502Config {
    pub frequency: Frequency,
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
    /// Next instruction that will be executed
    pub next_instruction: Option<Mos6502InstructionSet>,
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
            next_instruction: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RdyFlag(AtomicBool);

impl Default for RdyFlag {
    fn default() -> Self {
        Self(AtomicBool::new(true))
    }
}

impl RdyFlag {
    pub fn load(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    pub fn store(&self, value: bool) {
        self.0.store(value, Ordering::Release)
    }
}

#[derive(Debug)]
pub struct Mos6502 {
    state: ProcessorState,
    rdy: Arc<RdyFlag>,
    irq: Arc<IrqFlag>,
    nmi: Arc<NmiFlag>,
    config: Mos6502Config,
    address_space: Arc<AddressSpace>,
    address_space_cache: AddressSpaceCache,
    timestamp: Period,
    period: Period,
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

    pub fn address_space(&self) -> AddressSpaceId {
        self.config.assigned_address_space
    }

    #[inline]
    fn fetch_and_decode(&mut self) {
        let byte: u8 = self
            .address_space
            .read_le_value(
                self.state.program as Address,
                self.timestamp,
                Some(&mut self.address_space_cache),
            )
            .unwrap_or_default();

        let instruction_identifier = InstructionGroup::from_repr(byte & 0b11).unwrap();
        let secondary_instruction_identifier = (byte >> 5) & 0b111;
        let argument = (byte >> 2) & 0b111;

        let (opcode, addressing_mode) = match instruction_identifier {
            InstructionGroup::Group3 => decode_group3_space_instruction(
                secondary_instruction_identifier,
                argument,
                self.config.kind,
            ),
            InstructionGroup::Group1 => decode_group1_space_instruction(
                secondary_instruction_identifier,
                argument,
                self.config.kind,
            ),
            InstructionGroup::Group2 => decode_group2_space_instruction(
                secondary_instruction_identifier,
                argument,
                self.config.kind,
            ),
            InstructionGroup::Undocumented => decode_undocumented_space_instruction(
                secondary_instruction_identifier,
                argument,
                self.config.kind,
            ),
        };

        let instruction = Mos6502InstructionSet {
            opcode,
            addressing_mode,
        };

        debug_assert!(
            instruction.addressing_mode.is_none_or(|addressing_mode| {
                addressing_mode.is_valid_for_mode(self.config.kind)
            }),
            "Invalid addressing mode for instruction for mode {:?}: {:?}",
            self.config.kind,
            instruction,
        );

        self.state.address_bus = self.state.program.wrapping_add(1);
        self.state.program = self.state.program.wrapping_add(
            1 + instruction
                .addressing_mode
                .map_or(0, |mode| mode.added_instruction_length()),
        );
        self.state.latch.clear();

        self.push_steps_for_instruction(&instruction);

        self.state.next_instruction = Some(instruction);
        self.state
            .execution_queue
            .push_back(ExecutionStep::Interpret);
    }

    fn push_steps_for_instruction(&mut self, instruction: &Mos6502InstructionSet) {
        if let Some(addressing_mode) = instruction.addressing_mode {
            match addressing_mode {
                AddressingMode::Mos6502(Mos6502AddressingMode::Absolute) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::Immediate) => {}
                AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus(AddressBusModification::X),
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus(AddressBusModification::Y),
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::AbsoluteIndirect) => {
                    if self.config.kind.absolute_indirect_page_wrap_errata() {
                        self.state.execution_queue.extend([
                            ExecutionStep::LoadData,
                            ExecutionStep::LoadData,
                            ExecutionStep::LatchToAddressBus,
                            ExecutionStep::LoadDataWithoutAdvancingPage,
                            ExecutionStep::LoadData,
                            ExecutionStep::LatchToAddressBus,
                        ]);
                    } else {
                        self.state.execution_queue.extend([
                            ExecutionStep::LoadData,
                            ExecutionStep::LoadData,
                            ExecutionStep::LatchToAddressBus,
                            ExecutionStep::LoadData,
                            ExecutionStep::LoadData,
                            ExecutionStep::LatchToAddressBus,
                        ]);
                    }
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPageIndirect) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus(AddressBusModification::X),
                        ExecutionStep::MaskAddressBusToZeroPage,
                        ExecutionStep::LoadData,
                        ExecutionStep::MaskAddressBusToZeroPage,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPageIndirectYIndexed) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::LoadData,
                        ExecutionStep::MaskAddressBusToZeroPage,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus(AddressBusModification::Y),
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus(AddressBusModification::X),
                        ExecutionStep::MaskAddressBusToZeroPage,
                    ]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedZeroPage) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::ModifyAddressBus(AddressBusModification::Y),
                        ExecutionStep::MaskAddressBusToZeroPage,
                    ]);
                }

                AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPage) => {
                    self.state
                        .execution_queue
                        .extend([ExecutionStep::LoadData, ExecutionStep::LatchToAddressBus]);
                }
                AddressingMode::Mos6502(Mos6502AddressingMode::Relative) => {}
                AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator) => {}
                AddressingMode::Wdc65C02(Wdc65C02AddressingMode::ZeroPageIndirect) => {
                    self.state.execution_queue.extend([
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                        ExecutionStep::LoadData,
                        ExecutionStep::MaskAddressBusToZeroPage,
                        ExecutionStep::LoadData,
                        ExecutionStep::LatchToAddressBus,
                    ]);
                }
            }
        }
    }
}

impl Component for Mos6502 {
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        rmp_serde::encode::write_named(
            &mut writer,
            &Snapshot {
                state: self.state.clone(),
                rdy: self.rdy.load(),
            },
        )?;

        Ok(())
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match version {
            0 => {
                let snapshot: Snapshot = rmp_serde::decode::from_read(reader)?;

                self.state = snapshot.state;
                self.rdy.store(snapshot.rdy);

                Ok(())
            }
            other => Err(format!("Unsupported snapshot version: {other}").into()),
        }
    }

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        for now in context.allocate(self.period, None) {
            self.timestamp = now;

            if self.rdy.load() {
                loop {
                    match self.state.execution_queue.pop_front().unwrap() {
                        ExecutionStep::Reset => {
                            self.state.interrupt(RESET_VECTOR, false, false);

                            break;
                        }
                        ExecutionStep::Jammed => {
                            self.state.execution_queue.clear();
                            self.state.execution_queue.push_back(ExecutionStep::Jammed);

                            break;
                        }
                        ExecutionStep::Wait => {
                            self.state.execution_queue.push_back(ExecutionStep::Wait);

                            break;
                        }
                        ExecutionStep::FetchAndDecode => {
                            if self.config.kind.supports_interrupts() {
                                if self.nmi.interrupt_required() {
                                    self.state.interrupt(NMI_VECTOR, false, true);
                                } else if self.irq.interrupt_required()
                                    && !self.state.flags.interrupt_disable
                                {
                                    self.state.interrupt(IRQ_VECTOR, true, true);
                                } else {
                                    self.fetch_and_decode();
                                }
                            } else {
                                self.fetch_and_decode();
                            }

                            break;
                        }
                        ExecutionStep::LoadData => {
                            let byte = self
                                .address_space
                                .read_le_value(
                                    self.state.address_bus as usize,
                                    self.timestamp,
                                    Some(&mut self.address_space_cache),
                                )
                                .unwrap_or_default();

                            self.state.latch.push(byte);
                            self.state.address_bus = self.state.address_bus.wrapping_add(1);

                            break;
                        }
                        ExecutionStep::LoadDataWithoutAdvancingPage => {
                            let byte = self
                                .address_space
                                .read_le_value(
                                    self.state.address_bus as usize,
                                    self.timestamp,
                                    Some(&mut self.address_space_cache),
                                )
                                .unwrap_or_default();

                            self.state.latch.push(byte);

                            let mut address_bus_contents = self.state.address_bus.to_le_bytes();
                            address_bus_contents[0] = address_bus_contents[0].wrapping_add(1);
                            self.state.address_bus = u16::from_le_bytes(address_bus_contents);

                            break;
                        }
                        ExecutionStep::LoadDataFromConstant(data) => {
                            self.state.latch.push(data);

                            break;
                        }
                        ExecutionStep::StoreData(data) => {
                            let _ = self.address_space.write_le_value(
                                self.state.address_bus as usize,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                                data,
                            );
                            self.state.address_bus = self.state.address_bus.wrapping_add(1);

                            break;
                        }
                        ExecutionStep::PushStack(data) => {
                            let _ = self.address_space.write_le_value(
                                STACK_BASE_ADDRESS + self.state.stack as usize,
                                self.timestamp,
                                Some(&mut self.address_space_cache),
                                data,
                            );
                            self.state.stack = self.state.stack.wrapping_sub(1);

                            break;
                        }
                        ExecutionStep::LatchToAddressBus => {
                            match self.state.latch.len() {
                                1 => {
                                    self.state.address_bus = u16::from(self.state.latch[0]);
                                }
                                2 => {
                                    let latch = [self.state.latch[0], self.state.latch[1]];
                                    self.state.address_bus = u16::from_le_bytes(latch);
                                }
                                _ => {
                                    unreachable!()
                                }
                            }

                            self.state.latch.clear();
                        }
                        // only used for interrupts
                        ExecutionStep::LatchToProgramPointer => {
                            assert!(self.state.latch.len() == 2);

                            self.state.program =
                                u16::from_le_bytes([self.state.latch[0], self.state.latch[1]]);
                            self.state.latch.clear();
                        }
                        ExecutionStep::AddressBusToProgramPointer => {
                            self.state.program = self.state.address_bus;

                            break;
                        }
                        ExecutionStep::ModifyProgramPointer(value) => {
                            self.state.program =
                                self.state.program.wrapping_add_signed(i16::from(value));

                            break;
                        }
                        ExecutionStep::MaskAddressBusToZeroPage => {
                            self.state.address_bus %= PAGE_SIZE as u16;
                        }
                        ExecutionStep::ModifyAddressBus(modification) => {
                            let modification = match modification {
                                AddressBusModification::X => self.state.x,
                                AddressBusModification::Y => self.state.y,
                            };

                            self.state.address_bus =
                                self.state.address_bus.wrapping_add(u16::from(modification));
                        }
                        ExecutionStep::Interpret => {
                            let instruction = self.state.next_instruction.take().unwrap();

                            self.interpret_instruction(instruction);

                            self.state
                                .execution_queue
                                .push_back(ExecutionStep::FetchAndDecode);

                            break;
                        }
                    }
                }
            }
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= self.period
    }
}

impl<P: Platform> ComponentConfig<P> for Mos6502Config {
    type Component = Mos6502;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let address_space = component_builder
            .get_address_space(self.assigned_address_space)
            .clone();

        component_builder.set_scheduler_participation(SchedulerParticipation::SchedulerDriven);

        Ok(Mos6502 {
            rdy: Arc::default(),
            irq: Arc::default(),
            nmi: Arc::default(),
            state: ProcessorState::default(),
            address_space_cache: address_space.cache(),
            address_space,
            period: self.frequency.recip(),
            config: self,
            timestamp: Period::default(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    state: ProcessorState,
    rdy: bool,
}

#[derive(Debug)]
pub struct IrqFlag(AtomicBool);

impl Default for IrqFlag {
    fn default() -> Self {
        Self(AtomicBool::new(true))
    }
}

impl IrqFlag {
    pub fn store(&self, irq: bool) {
        self.0.store(irq, Ordering::Release);
    }

    pub fn interrupt_required(&self) -> bool {
        !self.0.load(Ordering::Acquire)
    }
}

/// NMI is falling edge
#[derive(Debug)]
pub struct NmiFlag {
    current_state: AtomicBool,
    falling_edge_occured: AtomicBool,
}

impl Default for NmiFlag {
    fn default() -> Self {
        Self {
            current_state: AtomicBool::new(true),
            falling_edge_occured: AtomicBool::new(false),
        }
    }
}

impl NmiFlag {
    pub fn store(&self, nmi: bool) {
        if self.current_state.swap(nmi, Ordering::AcqRel) && !nmi {
            self.falling_edge_occured.store(true, Ordering::Release);
        }
    }

    pub fn interrupt_required(&self) -> bool {
        self.falling_edge_occured.swap(false, Ordering::AcqRel)
    }
}

impl ProcessorState {
    pub fn interrupt(&mut self, vector: u16, break_status: bool, save_current_state: bool) {
        let vector = vector.to_le_bytes();
        let mut flags = self.flags;

        flags.break_ = break_status;
        flags.undocumented = true;
        flags.interrupt_disable = true;

        if save_current_state {
            let program_pointer = self.program.to_le_bytes();

            self.execution_queue.extend([
                ExecutionStep::PushStack(program_pointer[1]),
                ExecutionStep::PushStack(program_pointer[0]),
                ExecutionStep::PushStack(flags.to_byte()),
            ]);
        }

        self.execution_queue.extend([
            ExecutionStep::LoadDataFromConstant(vector[0]),
            ExecutionStep::LoadDataFromConstant(vector[1]),
            ExecutionStep::LatchToAddressBus,
            ExecutionStep::LoadData,
            ExecutionStep::LoadData,
            ExecutionStep::LatchToProgramPointer,
            ExecutionStep::FetchAndDecode,
        ]);
    }
}
