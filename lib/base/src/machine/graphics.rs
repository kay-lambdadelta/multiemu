use crate::graphics::GraphicsApi;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::BitOr};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum DisplayOrientation {
    Center,
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
