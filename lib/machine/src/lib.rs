use crate::component::store::ComponentStore;
use component::ComponentId;
use display::{FrameReceptacle, backend::RenderBackend};
use input::{VirtualGamepadId, virtual_gamepad::VirtualGamepad};
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_rom::system::GameSystem;
use nalgebra::SVector;
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
    scheduler: Scheduler,
    component_store: Arc<ComponentStore>,
    memory_translation_table: Arc<MemoryTranslationTable>,
    virtual_gamepads: HashMap<VirtualGamepadId, Arc<VirtualGamepad>>,
    frame_receptacles: HashMap<ComponentId, Arc<FrameReceptacle<R>>>,
    game_system: GameSystem,
}

impl<R: RenderBackend> Machine<R> {
    pub fn memory_translation_table(&self) -> Arc<MemoryTranslationTable> {
        self.memory_translation_table.clone()
    }

    #[must_use]
    pub fn run(
        &mut self,
        last_frame_time: Duration,
        last_frame_rendering_time: Duration,
    ) -> RunOutput<R> {
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

        let screens = self
            .frame_receptacles
            .iter()
            .filter_map(|(id, receptacle)| receptacle.get().map(|framebuffer| (*id, framebuffer)))
            .collect();

        let audio = Vec::default();

        RunOutput { screens, audio }
    }

    pub fn virtual_gamepads(&self) -> &HashMap<VirtualGamepadId, Arc<VirtualGamepad>> {
        &self.virtual_gamepads
    }

    pub fn system(&self) -> GameSystem {
        self.game_system
    }
}

pub struct RunOutput<R: RenderBackend> {
    /// Screens that had a new submission last pass
    pub screens: HashMap<ComponentId, R::ComponentFramebuffer>,
    /// Audio data collected last pass
    pub audio: Vec<SVector<f32, 2>>,
}
