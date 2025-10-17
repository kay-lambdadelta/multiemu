mod graphics;
mod id;
mod info;
mod manager;

pub use id::*;
pub use info::*;
pub use manager::{ProgramMetadata, *};

#[derive(Debug, Clone)]
pub struct ProgramSpecification {
    pub id: ProgramId,
    pub info: ProgramInfo,
}
