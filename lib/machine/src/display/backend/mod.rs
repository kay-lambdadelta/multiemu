use arc_swap::ArcSwap;
use multiemu_config::graphics::GraphicsApi;
use std::{any::Any, fmt::Debug, sync::Arc};

#[cfg(all(feature = "vulkan", platform_desktop))]
pub mod vulkan;

pub mod software;

/// Trait for marker structs representing rendering backends
pub trait RenderApi: Default + Debug + Any + Sized + 'static {
    const GRAPHICS_API: GraphicsApi;
    type ComponentInitializationData: Debug + 'static;
    type ComponentFramebufferInner: Debug + 'static;
    type ContextExtensionSpecification: ContextExtensionSpecification;
}

pub trait ContextExtensionSpecification: Any + Default + Clone + 'static {
    fn combine(self, other: Self) -> Self
    where
        Self: Sized;
}

#[allow(type_alias_bounds)]
#[derive(Debug)]
pub struct ComponentFramebuffer<R: RenderApi>(
    Arc<ArcSwap<<R as RenderApi>::ComponentFramebufferInner>>,
);

impl<R: RenderApi> Clone for ComponentFramebuffer<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: RenderApi> ComponentFramebuffer<R> {
    pub fn new(value: Arc<<R as RenderApi>::ComponentFramebufferInner>) -> Self {
        Self(Arc::new(ArcSwap::from(value)))
    }

    pub fn load(&self) -> Arc<<R as RenderApi>::ComponentFramebufferInner> {
        self.0.load_full()
    }

    pub fn store(&self, value: Arc<<R as RenderApi>::ComponentFramebufferInner>) {
        self.0.store(value);
    }
}
