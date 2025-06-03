use crate::{
    builder::ComponentBuilder, display::backend::RenderApi,
    memory::memory_translation_table::MemoryTranslationTable,
};
use multiemu_rom::manager::RomManager;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, OnceLock},
};

pub mod component_ref;
pub(crate) mod store;

/// Stuff every component optionally needs
#[derive(Debug)]
pub struct RuntimeEssentials<R: RenderApi> {
    pub rom_manager: Arc<RomManager>,
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    pub render_initialization_data: OnceLock<R::ComponentInitializationData>,
}

// Basic supertrait for all components
#[allow(unused)]
pub trait Component: Debug + Any {
    /// Called when machine initialization is finished
    fn on_machine_ready(&self) {}

    /// Called when a frame is about to be rendered, so things like locks on video stuff can be dropped
    fn on_frame_start(&self) {}

    /// Reset state
    fn reset(&self) {}
}

// An initializable component

#[allow(unused)]
pub trait ComponentConfig<R: RenderApi>: Debug + Send + Sync + Sized {
    /// Paramters to create this component
    type Component: Component;

    /// Make a new component from the config
    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>);
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ComponentId(pub u16);
