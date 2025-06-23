use multiemu_graphics::GraphicsApi;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum DisplayOrientation {
    Center,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct DisplayId(pub u16);

pub trait DisplayCallback<G: GraphicsApi>: Send + Sync + Debug + 'static {
    fn access_framebuffer(&self, callback: Box<dyn FnOnce(&G::FramebufferTexture) + '_>);
}

pub struct DisplayInfo<G: GraphicsApi> {
    pub callback: Box<dyn DisplayCallback<G>>,
}

impl<G: GraphicsApi> Debug for DisplayInfo<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DisplayInfo").finish()
    }
}

#[derive(Debug)]
pub struct GraphicsRequirements<G: GraphicsApi> {
    pub required_features: G::Features,
    pub preferred_features: G::Features,
}

impl<G: GraphicsApi> Clone for GraphicsRequirements<G> {
    fn clone(&self) -> Self {
        Self {
            required_features: self.required_features.clone(),
            preferred_features: self.preferred_features.clone(),
        }
    }
}

impl<G: GraphicsApi> Default for GraphicsRequirements<G> {
    fn default() -> Self {
        Self {
            required_features: Default::default(),
            preferred_features: Default::default(),
        }
    }
}
