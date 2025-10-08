use std::{any::Any, fmt::Debug, ops::BitOr};

#[cfg(feature = "opengl")]
pub mod opengl;
pub mod software;
#[cfg(feature = "vulkan")]
pub mod vulkan;

/// Trait for marker structs representing rendering backends
pub trait GraphicsApi: Default + Debug + Any + Sized + Send + Sync + 'static {
    /// Data components need to do their graphics operations
    type InitializationData: Clone + Debug + 'static;
    /// The component framebuffer type
    type FramebufferTexture: Send + Sync + Debug + 'static;
    /// How components describe what they require out of a graphics context
    type Features: Default + BitOr<Output = Self::Features> + Clone + Debug + 'static;
}
