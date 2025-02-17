use crate::builder::MachineBuilder;
use crate::component::store::ComponentStore;
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_config::Environment;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::GameSystem;
use scheduler::Scheduler;
use std::sync::{Arc, RwLock};

pub mod builder;
pub mod component;
pub mod display;
pub mod input;
pub mod memory;
pub mod message;
pub mod processor;
pub mod scheduler;

pub struct Machine {
    memory_translation_table: Arc<MemoryTranslationTable>,
    component_store: Arc<ComponentStore>,
    scheduler: Scheduler,
}

impl Machine {
    pub fn build(
        system: GameSystem,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) -> MachineBuilder {
        MachineBuilder::new(system, rom_manager, environment)
    }

    pub fn memory_translation_table(&self) -> Arc<MemoryTranslationTable> {
        self.memory_translation_table.clone()
    }

    pub fn component_store(&self) -> &Arc<ComponentStore> {
        &self.component_store
    }
}
