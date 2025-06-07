//! Main graphics definition things for multiemu

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

mod backend;
#[cfg(any(feature = "vulkan", feature = "opengl"))]
pub mod shader;

pub use backend::*;
use serde::{Deserialize, Serialize};

/// Description of what extensions are required and preferred from a graphics context
pub struct GraphicsContextExtensions<R: GraphicsApi> {
    /// Extensions that are absolutely needed
    pub required: R::ContextExtensionSpecification,
    /// Extensions that are wanted and could operate without
    pub preferred: R::ContextExtensionSpecification,
}

impl<R: GraphicsApi> Default for GraphicsContextExtensions<R> {
    fn default() -> Self {
        Self {
            required: Default::default(),
            preferred: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Version specifier used in this library
pub struct GraphicsVersion {
    /// Major
    pub major: u32,
    /// Minor
    pub minor: u32,
}
