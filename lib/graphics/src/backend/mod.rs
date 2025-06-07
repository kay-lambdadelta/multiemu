use core::{any::Any, fmt::Debug};

#[cfg(feature = "opengl")]
mod opengl;
mod software;
#[cfg(feature = "vulkan")]
mod vulkan;

#[cfg(feature = "vulkan")]
pub use opengl::*;
pub use software::*;
#[cfg(feature = "opengl")]
pub use vulkan::*;

/// Trait for marker structs representing rendering backends
pub trait GraphicsApi: Default + Debug + Any + Sized + 'static {
    /// Data components need to do their graphics operations
    type ComponentGraphicsInitializationData: Debug + 'static;
    /// The component framebuffer type
    type ComponentFramebuffer: Debug + 'static;
    /// How components describe what they require out of a graphics context
    type ContextExtensionSpecification: ContextExtensionSpecification;
}

/// Trait for context extensions
pub trait ContextExtensionSpecification: Any + Debug + Default + Clone + 'static {
    /// Combine the extensions together (or operation)
    fn combine(self, other: Self) -> Self
    where
        Self: Sized;
}
