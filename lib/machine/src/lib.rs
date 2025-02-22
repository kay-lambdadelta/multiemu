use crate::component::store::ComponentStore;
use component::ComponentId;
use crossbeam::channel::Receiver;
use display::RenderBackend;
use input::{VirtualGamepadId, virtual_gamepad::VirtualGamepad};
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_rom::system::GameSystem;
use scheduler::Scheduler;
use std::{collections::HashMap, sync::Arc};

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
    virtual_gamepads: HashMap<VirtualGamepadId, Arc<VirtualGamepad>>,
    game_system: GameSystem,
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

    pub fn virtual_gamepads(&self) -> &HashMap<VirtualGamepadId, Arc<VirtualGamepad>> {
        &self.virtual_gamepads
    }

    pub fn game_system(&self) -> GameSystem {
        self.game_system
    }
}
