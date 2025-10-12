use multiemu_base::processor::InstructionSet;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Range};
use strum::FromRepr;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, FromRepr,
)]
#[repr(u8)]
pub enum Register {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    VA,
    VB,
    VC,
    VD,
    VE,
    VF,
}

// https://github.com/craigthomas/Chip8Assembler
// TODO: These mnemonics are terrible

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstructionSetChip8 {
    Clr,
    Rtrn,
    Jump {
        address: u16,
    },
    /// Jump but it adds the program counter to the stack
    Call {
        address: u16,
    },
    Ske {
        register: Register,
        immediate: u8,
    },
    Skne {
        register: Register,
        immediate: u8,
    },
    Skre {
        param_1: Register,
        param_2: Register,
    },
    Load {
        register: Register,
        immediate: u8,
    },
    Add {
        register: Register,
        immediate: u8,
    },
    Move {
        param_1: Register,
        param_2: Register,
    },
    Or {
        destination: Register,
        source: Register,
    },
    And {
        destination: Register,
        source: Register,
    },
    Xor {
        destination: Register,
        source: Register,
    },
    Addr {
        destination: Register,
        source: Register,
    },
    Sub {
        destination: Register,
        source: Register,
    },
    Shr {
        register: Register,
        value: Register,
    },
    Subn {
        destination: Register,
        source: Register,
    },
    Shl {
        register: Register,
        value: Register,
    },
    Skrne {
        param_1: Register,
        param_2: Register,
    },
    Loadi {
        value: u16,
    },
    Jumpi {
        address: u16,
    },
    Rand {
        register: Register,
        immediate: u8,
    },
    Draw {
        coordinates: Point2<Register>,
        height: u8,
    },
    Skpr {
        key: Register,
    },
    Skup {
        key: Register,
    },
    Moved {
        register: Register,
    },
    Keyd {
        key: Register,
    },
    Loadd {
        register: Register,
    },
    Loads {
        register: Register,
    },
    Addi {
        register: Register,
    },
    Font {
        register: Register,
    },
    Bcd {
        register: Register,
    },
    Save {
        count: u8,
    },
    Restore {
        count: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScrollDirection {
    Left,
    Right,
    Down { amount: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstructionSetSuperChip8 {
    Lores,
    Hires,
    Scroll { direction: ScrollDirection },
    Scrd { amount: u8 },
    Scrr,
    Scrl,
    Srpl { amount: u8 },
    Rrpl { amount: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstructionSetXoChip {
    Ssub { bounds: Range<Register> },
    Rsub { bounds: Range<Register> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Chip8InstructionSet {
    Chip8(InstructionSetChip8),
    SuperChip8(InstructionSetSuperChip8),
    XoChip(InstructionSetXoChip),
}

impl Display for Chip8InstructionSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl InstructionSet for Chip8InstructionSet {
    type Opcode = Self;
    type AddressingMode = ();
}
