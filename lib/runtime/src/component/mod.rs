use crate::{
    builder::ComponentBuilder, component::component_ref::ComponentRef,
    memory::memory_translation_table::MemoryTranslationTable,
};
use multiemu_graphics::GraphicsApi;
use multiemu_rom::manager::RomManager;
use num::rational::Ratio;
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
pub struct RuntimeEssentials<R: GraphicsApi> {
    pub rom_manager: Arc<RomManager>,
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    /// This is not guarenteed to be initialized until [Component::on_runtime_ready] is called
    ///
    /// Therefore do not expect it to be filled out until then
    pub component_graphics_initialization_data: OnceLock<R::ComponentGraphicsInitializationData>,
    pub sample_rate: Ratio<u32>,
}

// Basic supertrait for all components
#[allow(unused)]
pub trait Component: Debug + Any {
    /// Called when machine initialization is finished
    ///
    /// This is where you should do graphics initialization or anything that reads or writes from the memory translation table
    fn on_runtime_ready(&self) {}

    /// Reset state
    fn reset(&self) {}
}

// An initializable component

#[allow(unused)]
pub trait ComponentConfig<B: ComponentBuilder<Component = Self::Component>>:
    Debug + Send + Sync + Sized
{
    /// Paramters to create this component
    type Component: Component;

    /// Make a new component from the config
    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: B,
    ) -> B::BuildOutput;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ComponentId(pub u16);
