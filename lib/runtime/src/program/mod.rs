mod graphics;
mod id;
mod info;
mod manager;

pub use id::*;
pub use info::*;
pub use manager::{ProgramMetadata, *};

/// Identifier for the emulator to recognize a program as unique and info on it
#[derive(Debug, Clone)]
pub struct ProgramSpecification {
    /// Id
    pub id: ProgramId,
    /// (Usually) database derived information
    pub info: ProgramInfo,
}
