use crate::{
    ComponentStore, builder::ComponentBuilder, display::shader::ShaderCache,
    memory::memory_translation_table::MemoryTranslationTable,
};
use downcast_rs::Downcast;
use multiemu_config::Environment;
use multiemu_rom::manager::RomManager;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    any::Any,
    fmt::Debug,
    io::{Read, Write},
    sync::{Arc, RwLock},
};
use versions::SemVer;

pub mod component_ref;
pub mod main_thread_queue;
pub mod store;

/// Stuff every component optionally needs
#[derive(Debug)]
pub struct RuntimeEssentials {
    pub component_store: Arc<ComponentStore>,
    pub rom_manager: Arc<RomManager>,
    pub environment: Arc<RwLock<Environment>>,
    pub shader_cache: Arc<ShaderCache>,
    pub memory_translation_table: MemoryTranslationTable,
}

// Basic supertrait for all components
#[allow(unused)]
pub trait Component: Any + Downcast {
    fn reset(&self) {}
    fn save(&self, entry: &mut dyn Write) -> Result<SemVer, Box<dyn std::error::Error>> {
        Ok(SemVer::new("1.0.0").unwrap())
    }
    fn load(
        &self,
        entry: &mut dyn Read,
        version: SemVer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// An initializable component
pub trait FromConfig: Component + Sized {
    /// Paramters to create this component
    type Config: Debug + Send + Sync;
    /// ROM specific behavior changes this component should apply
    type Quirks: Serialize + DeserializeOwned + Default + Debug + Send + Sync;

    /// Make a new component from the config
    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        quirks: Self::Quirks,
    );
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub u16);
