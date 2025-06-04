use super::ContextExtensionSpecification;
use crate::display::{
    RenderApi,
    shader::{ShaderCache, glsl::GlslShader},
};
use glow::{Context, Framebuffer};
use nalgebra::Vector2;
use std::rc::Rc;

#[derive(Default, Debug)]
pub struct OpenglRendering;

#[derive(Debug, Default, Clone)]
pub struct OpenglContextExtensionSpecification;

impl ContextExtensionSpecification for OpenglContextExtensionSpecification {
    fn combine(self, _other: Self) -> Self
    where
        Self: Sized,
    {
        Self
    }
}

impl RenderApi for OpenglRendering {
    type ComponentInitializationData = OpenglComponentInitializationData;
    type ComponentFramebufferInner = OpenglFramebuffer;
    type ContextExtensionSpecification = OpenglContextExtensionSpecification;
}

#[derive(Debug)]
pub struct OpenglFramebuffer {
    pub dimensions: Vector2<u16>,
    pub image: Framebuffer,
}

#[derive(Debug)]
pub struct OpenglComponentInitializationData {
    pub context: Rc<Context>,
    pub shader_cache: ShaderCache<GlslShader>,
}
