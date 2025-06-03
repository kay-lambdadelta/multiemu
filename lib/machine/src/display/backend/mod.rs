use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, RwLock},
};

#[cfg(feature = "opengl")]
pub mod opengl;
pub mod software;
#[cfg(feature = "vulkan")]
pub mod vulkan;

/// Trait for marker structs representing rendering backends
pub trait RenderApi: Default + Debug + Any + Sized + 'static {
    type ComponentInitializationData: Debug + 'static;
    type ComponentFramebufferInner: Debug + 'static;
    type ContextExtensionSpecification: ContextExtensionSpecification;
}

pub trait ContextExtensionSpecification: Any + Default + Clone + 'static {
    fn combine(self, other: Self) -> Self
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct ComponentFramebuffer<R: RenderApi>(
    Arc<RwLock<Arc<<R as RenderApi>::ComponentFramebufferInner>>>,
);

impl<R: RenderApi> Clone for ComponentFramebuffer<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: RenderApi> ComponentFramebuffer<R> {
    pub fn new(value: Arc<<R as RenderApi>::ComponentFramebufferInner>) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn load(&self) -> Arc<<R as RenderApi>::ComponentFramebufferInner> {
        self.0.read().unwrap().clone()
    }

    pub fn store(&self, value: Arc<<R as RenderApi>::ComponentFramebufferInner>) {
        *self.0.write().unwrap() = value
    }
}
