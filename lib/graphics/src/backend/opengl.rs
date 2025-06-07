use super::ContextExtensionSpecification;
use crate::{
    GraphicsApi,
    shader::{GlslShader, ShaderCache},
};
use alloc::rc::Rc;
use glow::{Context, Framebuffer};
use nalgebra::Vector2;

#[derive(Default, Debug)]
/// Marker train for opengl rendering, supported on major desktop platforms, lesser to the vulkan one
pub struct Opengl;

#[derive(Debug, Default, Clone)]
/// TODO: Actually fill this out
pub struct OpenglContextExtensionSpecification;

impl ContextExtensionSpecification for OpenglContextExtensionSpecification {
    fn combine(self, _other: Self) -> Self
    where
        Self: Sized,
    {
        Self
    }
}

impl GraphicsApi for Opengl {
    type ComponentGraphicsInitializationData = OpenglComponentInitializationData;
    type ComponentFramebuffer = OpenglFramebuffer;
    type ContextExtensionSpecification = OpenglContextExtensionSpecification;
}

#[derive(Debug)]
/// A framebuffer and its dimensions
pub struct OpenglFramebuffer {
    /// Dimensions
    pub dimensions: Vector2<u16>,
    /// The actual handle to the framebuffer
    pub image: Framebuffer,
}

#[derive(Debug)]
/// opengl initialization data
pub struct OpenglComponentInitializationData {
    /// Graphics context
    pub context: Rc<Context>,
    /// Shader cache for glsl
    pub shader_cache: ShaderCache<GlslShader>,
}
