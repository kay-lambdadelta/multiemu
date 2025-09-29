mod decoder;
mod instruction;

// #[cfg(jit)]
// pub mod jit;

pub use decoder::InstructionDecoder;
pub use instruction::InstructionSet;
