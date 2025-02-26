use crate::ComponentStore;
use crate::builder::ComponentBuilder;
use crate::memory::memory_translation_table::MemoryTranslationTable;
use downcast_rs::Downcast;
use multiemu_config::Environment;
use multiemu_rom::manager::RomManager;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use std::sync::{Arc, OnceLock, RwLock, RwLockReadGuard};

pub mod component_ref;
pub mod store;

/// Stuff every component optionally needs
#[derive(Debug, Clone)]
pub struct RuntimeEssentials {
    memory_translation_table: OnceLock<Arc<MemoryTranslationTable>>,
    component_store: Arc<ComponentStore>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
}

impl RuntimeEssentials {
    pub(crate) fn new(rom_manager: Arc<RomManager>, environment: Arc<RwLock<Environment>>) -> Self {
        Self {
            memory_translation_table: OnceLock::new(),
            component_store: Arc::new(ComponentStore::default()),
            rom_manager,
            environment,
        }
    }

    pub(crate) fn set_memory_translation_table(
        &self,
        memory_translation_table: Arc<MemoryTranslationTable>,
    ) {
        self.memory_translation_table
            .set(memory_translation_table)
            .unwrap();
    }

    /// The memory translation table is late initalized so you cannot call this within componentbuilder
    pub fn memory_translation_table(&self) -> &Arc<MemoryTranslationTable> {
        self.memory_translation_table
            .get()
            .expect("Memory translation table not initialized yet")
    }

    pub fn component_store(&self) -> &Arc<ComponentStore> {
        &self.component_store
    }

    pub fn rom_manager(&self) -> &Arc<RomManager> {
        &self.rom_manager
    }

    pub fn environment(&self) -> RwLockReadGuard<'_, Environment> {
        self.environment.read().unwrap()
    }
}

// Basic supertrait for all components
pub trait Component: Any + Downcast {
    fn reset(&self) {}
}

// An initializable component
pub trait FromConfig: Component + Sized {
    /// Paramters to create this component
    type Config: Debug + Send + Sync;
    /// ROM specific behavior changes this component should apply
    type Quirks: Default + Debug + Send + Sync;

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
