use enumflags2::bitflags;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
};
use std::sync::Arc;

// mod decode;
// mod instruction;

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
enum Z80FlagRegister {
    Sign = 0b1000_0000,
    Zero = 0b0100_0000,
    __Unused0 = 0b0010_0000,
    HalfCarry = 0b0001_0000,
    __Unused1 = 0b0000_1000,
    Overflow = 0b0000_0100,
    Parity = 0b0000_0010,
    Carry = 0b0000_0001,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
enum Lr35902FlagRegister {
    Zero = 0b1000_0000,
    Subtract = 0b0100_0000,
    HalfCarry = 0b0010_0000,
    Carry = 0b0001_0000,
    __Unused0 = 0b0000_1000,
    __Unused1 = 0b0000_0100,
    __Unused2 = 0b0000_0010,
    __Unused3 = 0b0000_0001,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
enum I8080FlagRegister {
    Sign = 0b1000_0000,
    Zero = 0b0100_0000,
    __Unused0 = 0b0010_0000,
    AuxiliaryCarry = 0b0001_0000,
    __Unused1 = 0b0000_1000,
    Parity = 0b0000_0100,
    __Unused2 = 0b0000_0010,
    Carry = 0b0000_0001,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum I8080Kind {
    I8080,
    Z80,
    Lr35902,
}

#[derive(Debug)]
pub struct I8080 {
    config: I8080Config,
}

impl Component for I8080 {}

#[derive(Debug)]
pub struct I8080Config {
    pub kind: I8080Kind,
}

impl I8080Config {
    pub fn lr35902() -> Self {
        Self {
            kind: I8080Kind::Lr35902,
        }
    }

    pub fn z80() -> Self {
        Self {
            kind: I8080Kind::Z80,
        }
    }

    pub fn i8080() -> Self {
        Self {
            kind: I8080Kind::I8080,
        }
    }
}

impl FromConfig for I8080 {
    type Config = I8080Config;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
    ) {
        todo!()
    }
}
