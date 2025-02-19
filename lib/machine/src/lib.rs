use crate::component::store::ComponentStore;
use component::ComponentId;
use crossbeam::channel::Receiver;
use display::RenderBackend;
use memory::memory_translation_table::MemoryTranslationTable;
use scheduler::Scheduler;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

pub mod builder;
pub mod component;
pub mod display;
pub mod input;
pub mod memory;
pub mod message;
pub mod processor;
pub mod scheduler;

pub struct Machine<R: RenderBackend> {
    pub scheduler: Scheduler,
    component_store: Arc<ComponentStore>,
    memory_translation_table: Arc<MemoryTranslationTable>,
    component_framebuffers: HashMap<ComponentId, Receiver<R::ComponentFramebuffer>>,
}

impl<R: RenderBackend> Machine<R> {
    pub fn memory_translation_table(&self) -> &Arc<MemoryTranslationTable> {
        &self.memory_translation_table
    }

    pub fn component_store(&self) -> &Arc<ComponentStore> {
        &self.component_store
    }

    pub fn framebuffer_receivers(
        &self,
    ) -> &HashMap<ComponentId, Receiver<R::ComponentFramebuffer>> {
        &self.component_framebuffers
    }
}
