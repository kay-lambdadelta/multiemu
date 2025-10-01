//! Main graphics definition things for multiemu

mod backend;

pub use backend::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Version specifier used in this library
pub struct GraphicsVersion {
    /// Major
    pub major: u32,
    /// Minor
    pub minor: u32,
}

pub type ShaderBool = u32;
