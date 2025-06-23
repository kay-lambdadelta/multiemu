//! Main graphics definition things for multiemu

mod backend;
#[cfg(any(feature = "vulkan", feature = "opengl"))]
pub mod shader;

pub use backend::*;
use serde::{Deserialize, Serialize};

/// Description of what extensions are required and preferred from a graphics context
pub struct GraphicsContextFeatures<R: GraphicsApi> {
    /// Extensions that are absolutely needed
    pub required: R::Features,
    /// Extensions that are wanted and could operate without
    pub preferred: R::Features,
}

impl<R: GraphicsApi> Default for GraphicsContextFeatures<R> {
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
