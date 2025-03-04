use crate::component::store::ComponentStore;
use component::ComponentId;
use crossbeam::channel::Receiver;
use display::RenderBackend;
use input::{VirtualGamepadId, virtual_gamepad::VirtualGamepad};
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_rom::system::GameSystem;
use scheduler::Scheduler;
use std::{collections::HashMap, sync::Arc};

pub mod audio;
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
    pub component_store: Arc<ComponentStore>,
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    pub component_framebuffers: HashMap<ComponentId, Receiver<R::ComponentFramebuffer>>,
    pub virtual_gamepads: HashMap<VirtualGamepadId, Arc<VirtualGamepad>>,
    pub game_system: GameSystem,
}
