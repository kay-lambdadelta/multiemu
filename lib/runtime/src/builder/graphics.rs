use crate::graphics::GraphicsRequirements;
use multiemu_graphics::GraphicsApi;

pub struct GraphicsMetadata<G: GraphicsApi> {
    /// Callback for getting the texture
    pub graphics_requirements: GraphicsRequirements<G>,
}

impl<G: GraphicsApi> Default for GraphicsMetadata<G> {
    fn default() -> Self {
        Self {
            graphics_requirements: Default::default(),
        }
    }
}
