use crate::builder::ComponentBuilder;
use crate::memory::memory_translation_table::MemoryTranslationTable;
use crate::ComponentStore;
use downcast_rs::Downcast;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use std::sync::{Arc, OnceLock};

pub mod component_ref;
pub mod store;

/// Stuff every component optionally needs
#[derive(Debug, Clone)]
pub struct RuntimeEssentials {
    memory_translation_table: OnceLock<Arc<MemoryTranslationTable>>,
    component_store: Arc<ComponentStore>,
}

impl RuntimeEssentials {
    pub fn memory_translation_table(&self) -> &MemoryTranslationTable {
        self.memory_translation_table.get().unwrap()
    }

    pub fn component_store(&self) -> &ComponentStore {
        &self.component_store
    }
}

// Basic supertrait for all components
pub trait Component: Any + Downcast {
    fn reset(&self) {}
    fn set_essentials(&mut self, _essentials: RuntimeEssentials) {}
}

// An initializable component
pub trait FromConfig: Component + Sized {
    type Config: Debug + Send + Sync;

    /// Make a new component from the config
    fn from_config(component_builder: ComponentBuilder<Self>, config: Self::Config);
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub u16);
