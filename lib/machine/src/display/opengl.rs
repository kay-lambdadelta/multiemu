use crate::display::{ContextExtensionSpecification, RenderBackend};
use glium::backend::Context;
use glium::Texture2d;
use multiemu_config::graphics::GraphicsApi;
use std::rc::Rc;

pub struct OpenGlRendering;

#[derive(Default, Clone)]
pub struct OpenGlContextExtensionSpecification;

impl ContextExtensionSpecification for OpenGlContextExtensionSpecification {
    fn combine(self, other: Self) -> Self
    where
        Self: Sized,
    {
        Self
    }
}

pub type OpenGlComponentFramebuffer = Rc<Texture2d>;

impl RenderBackend for OpenGlRendering {
    const GRAPHICS_API: GraphicsApi = GraphicsApi::OpenGl;
    type ComponentInitializationData = OpenGlComponentInitializationData;
    type ComponentFramebuffer = OpenGlComponentFramebuffer;
    type ContextExtensionSpecification = OpenGlContextExtensionSpecification;
}

pub struct OpenGlComponentInitializationData {
    pub context: Rc<Context>,
}
