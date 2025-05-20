use enumflags2::bitflags;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Intel8080Kind {
    #[default]
    Intel8080,
    Zilog80,
    SharpLr35902,
}

#[derive(Debug)]
pub struct Intel8080 {
    config: Intel8080Config,
}

impl Component for Intel8080 {}

#[derive(Default, Debug)]
pub struct Intel8080Config {
    pub kind: Intel8080Kind,
}

impl Intel8080Config {
    pub fn lr35902() -> Self {
        Self {
            kind: Intel8080Kind::SharpLr35902,
        }
    }

    pub fn z80() -> Self {
        Self {
            kind: Intel8080Kind::Zilog80,
        }
    }

    pub fn i8080() -> Self {
        Self {
            kind: Intel8080Kind::Intel8080,
        }
    }
}

impl<R: RenderApi> ComponentConfig<R> for Intel8080Config {
    type Component = Intel8080;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        todo!()
    }
}
