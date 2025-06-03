use builder::MachineBuilder;
use display::backend::{ComponentFramebuffer, RenderApi};
use input::{VirtualGamepadId, virtual_gamepad::VirtualGamepad};
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_rom::{id::RomId, manager::RomManager, system::GameSystem};
use rustc_hash::FxBuildHasher;
use scheduler::Scheduler;
use std::{collections::HashMap, fmt::Debug, sync::Arc, vec::Vec};
use utils::Fragile;

pub mod audio;
pub mod builder;
pub mod component;
pub mod display;
pub mod input;
pub mod memory;
pub mod processor;
pub mod scheduler;
pub mod task;
pub mod utils;

#[non_exhaustive]
pub struct Machine<R: RenderApi> {
    pub scheduler: Scheduler,
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    pub virtual_gamepads: HashMap<VirtualGamepadId, Arc<VirtualGamepad>, FxBuildHasher>,
    pub game_system: GameSystem,
    pub framebuffers: Fragile<Vec<ComponentFramebuffer<R>>>,
}

pub trait MachineFactory<R: RenderApi>: Debug + Send + Sync + 'static {
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
    ) -> MachineBuilder<R>;
}
