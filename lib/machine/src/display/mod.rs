use multiemu_config::graphics::GraphicsApi;
use std::any::Any;

#[cfg(all(feature = "opengl", platform_desktop))]
pub mod opengl;
#[cfg(all(feature = "vulkan", platform_desktop))]
pub mod vulkan;

pub mod software;

/// Trait for marker structs representing rendering backends
pub trait RenderBackend: Any + 'static {
    const GRAPHICS_API: GraphicsApi;
    type ComponentInitializationData: 'static;
    type ComponentFramebuffer;
    type ContextExtensionSpecification: ContextExtensionSpecification;
}

pub trait ContextExtensionSpecification: Any + Default + Clone + 'static {
    fn combine(self, other: Self) -> Self
    where
        Self: Sized;
}
