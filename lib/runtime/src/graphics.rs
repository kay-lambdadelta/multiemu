use crate::{component::ComponentPath, utils::Fragile};
use arc_swap::ArcSwapOption;
use multiemu_graphics::GraphicsApi;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, fmt::Debug, ops::BitOr, sync::Arc};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub enum DisplayOrientation {
    Center,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct DisplayId {
    component_path: ComponentPath,
    name: Cow<'static, str>,
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

#[derive(Debug)]
pub struct FramebufferStorage<G: GraphicsApi> {
    displays: HashMap<DisplayId, ArcSwapOption<Fragile<G::FramebufferTexture>>>,
}

impl<G: GraphicsApi> Default for FramebufferStorage<G> {
    fn default() -> Self {
        Self {
            displays: Default::default(),
        }
    }
}

impl<G: GraphicsApi> FramebufferStorage<G> {
    pub fn reserve_entry(&mut self, id: DisplayId) {
        let overlap = self.displays.insert(id, ArcSwapOption::default()).is_some();

        if overlap {
            panic!("Overlapping id: {:?}", id);
        }
    }

    pub fn get(&self, display_id: &DisplayId) -> Arc<G::FramebufferTexture> {
        self.displays
            .get(display_id)
            .expect("Framebuffer not reserved")
            .load_full()
    }

    pub fn set(&self, display_id: DisplayId, display: Arc<G::FramebufferTexture>) {
        self.displays
            .get(&display_id)
            .expect("Framebuffer not reserved")
            .store(display);
    }
}
