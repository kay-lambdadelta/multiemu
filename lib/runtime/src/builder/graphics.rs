use crate::graphics::{DisplayId, DisplayInfo, GraphicsRequirements};
use multiemu_graphics::GraphicsApi;
use std::collections::HashMap;

pub struct GraphicsMetadata<G: GraphicsApi> {
    /// Callback for getting the texture
    pub displays: HashMap<DisplayId, DisplayInfo<G>>,
    pub graphics_requirements: GraphicsRequirements<G>,
}

impl<G: GraphicsApi> Default for GraphicsMetadata<G> {
    fn default() -> Self {
        Self {
            displays: HashMap::new(),
            graphics_requirements: Default::default(),
        }
    }
}
