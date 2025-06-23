use std::fmt::{Debug, Display};

/// The instruction set
pub trait InstructionSet: Debug + Eq + Clone + Send + Sync + 'static {
    /// Unique code representing an instruction
    type Opcode: Display + Debug + 'static;
    /// The specified addressing mode
    type AddressingMode: Debug + 'static;
}
