use crate::component::store::ComponentStore;
use component::ComponentId;
use display::backend::RenderBackend;
use input::{VirtualGamepadId, virtual_gamepad::VirtualGamepad};
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_rom::system::GameSystem;
use scheduler::Scheduler;
use std::{collections::HashMap, sync::Arc, time::Duration};

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
    pub virtual_gamepads: HashMap<VirtualGamepadId, Arc<VirtualGamepad>>,
    pub framebuffers: HashMap<ComponentId, R::ComponentFramebuffer>,
    pub game_system: GameSystem,
}

impl<R: RenderBackend> Machine<R> {
    pub fn run(&mut self, last_frame_time: Duration, last_frame_rendering_time: Duration) {
        let now = std::time::Instant::now();
        self.scheduler.run();
        let elapsed = now.elapsed();

        let alloted_time = last_frame_time - last_frame_rendering_time;

        match elapsed.cmp(&alloted_time) {
            std::cmp::Ordering::Less => {
                self.scheduler.speed_up();
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                self.scheduler.slow_down();
            }
        }
    }
}
