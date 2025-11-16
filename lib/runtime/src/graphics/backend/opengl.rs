use std::{ops::BitOr, rc::Rc};

pub use glow;
use glow::{Context, Framebuffer};
use nalgebra::Vector2;

use crate::{
    GraphicsApi,
    shader::{GlslShader, ShaderCache},
};

#[derive(Default, Debug)]
/// Marker train for opengl rendering, supported on major desktop platforms, lesser to the vulkan one
pub struct Opengl;

#[derive(Debug, Default, Clone)]
/// TODO: Actually fill this out
pub struct Features;

impl BitOr for Features {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        rhs
    }
}

impl GraphicsApi for Opengl {
    type InitializationData = InitializationData;
    type FramebufferTexture = FramebufferTexture;
    type Features = Features;
}

#[derive(Debug, Clone)]
/// A framebuffer and its dimensions
pub struct FramebufferTexture {
    /// Dimensions
    dimensions: Vector2<u32>,
    /// The actual handle to the framebuffer
    image: Framebuffer,
}

impl FramebufferTexture {
    pub fn dimensions(&self) -> Vector2<u32> {
        self.dimensions
    }

    pub fn image(&self) -> &Framebuffer {
        &self.image
    }
}

#[derive(Debug)]
/// opengl initialization data
pub struct InitializationData {
    /// Graphics context
    pub context: Rc<Context>,
    /// Shader cache for glsl
    pub shader_cache: ShaderCache<GlslShader>,
}
