use std::fmt::{Debug, Display};

pub trait InstructionSet: Debug + Eq + Clone + Send + Sync + 'static {
    type Opcode: Display + Debug + 'static;
    type AddressingMode: Debug + 'static;
}
