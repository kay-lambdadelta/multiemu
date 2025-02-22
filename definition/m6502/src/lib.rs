use enumflags2::{BitFlags, bitflags};
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable},
};
use num::rational::Ratio;
use std::{
    cell::{OnceCell, RefCell},
    sync::Arc,
};

pub mod decode;
pub mod instruction;
pub mod interpret;

#[cfg(test)]
pub mod test;

pub enum M6502Kind {
    /// Standard
    M6502 {
        /// Whether to emulated the broken ROR instruction
        quirk_broken_ror: bool,
    },
    /// Slimmed down atari 2600 version
    M6507,
    /// NES version
    R2A03,
    /// NES version
    R2A07,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
enum FlagRegister {
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
#[cfg_attr(jit, repr(C))]
pub struct M6502Registers {
    stack_pointer: u8,
    accumulator: u8,
    index_registers: [u8; 2],
    flags: BitFlags<FlagRegister>,
    program: u16,
}

#[derive(Debug)]
pub struct M6502Config {
    pub frequency: Ratio<u64>,
    pub assigned_address_space: AddressSpaceId,
}

#[derive(Debug)]
#[cfg_attr(jit, repr(C))]
struct ProcessorState {
    registers: M6502Registers,
    memory_translation_table: OnceCell<Arc<MemoryTranslationTable>>,
}

impl Default for ProcessorState {
    fn default() -> Self {
        Self {
            registers: M6502Registers {
                stack_pointer: 0xff,
                accumulator: 0,
                index_registers: [0, 0],
                flags: BitFlags::empty(),
                program: 0,
            },
            memory_translation_table: OnceCell::default(),
        }
    }
}

pub struct M6502 {
    config: M6502Config,
    state: RefCell<ProcessorState>,
    essentials: OnceCell<RuntimeEssentials>,
}

impl Component for M6502 {}

impl FromConfig for M6502 {
    type Config = M6502Config;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
    ) {
        let frequency = config.frequency;

        component_builder
            .insert_task(frequency, |_: &Self, _| {})
            .build(Self {
                config,
                state: RefCell::default(),
                essentials: OnceCell::default(),
            });
    }
}
