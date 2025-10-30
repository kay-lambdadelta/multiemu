use crate::graphics::GraphicsApi;
use std::{fmt::Debug, ops::BitOr};

/// The requirements for a graphics context
#[derive(Debug)]
pub struct GraphicsRequirements<G: GraphicsApi> {
    /// Features that are needed for rendering to occur
    pub required_features: G::Features,
    /// Features that would be nice to have
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

impl<G: GraphicsApi> BitOr for GraphicsRequirements<G> {
    type Output = GraphicsRequirements<G>;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            required_features: self.required_features | rhs.required_features,
            preferred_features: self.preferred_features | rhs.preferred_features,
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
