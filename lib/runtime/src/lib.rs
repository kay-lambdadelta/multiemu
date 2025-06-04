use builder::MachineBuilder;
use display::backend::{ComponentFramebuffer, RenderApi};
use input::{VirtualGamepadId, virtual_gamepad::VirtualGamepad};
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_rom::{id::RomId, manager::RomManager, system::GameSystem};
use rustc_hash::FxBuildHasher;
use scheduler::Scheduler;
use std::{any::Any, collections::HashMap, fmt::Debug, sync::Arc, vec::Vec};
use utils::Fragile;
use crate::audio::{sample::Sample, AudioDataCallback};

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

#[derive(Debug)]
pub struct Machine
where
    Self: Send + Sync,
{
    pub scheduler: Scheduler,
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    pub virtual_gamepads: HashMap<VirtualGamepadId, Arc<VirtualGamepad>, FxBuildHasher>,
    pub game_system: GameSystem,
    audio_data_callbacks: Box<dyn Any + Send + Sync>,
    framebuffers: Fragile<Box<dyn Any>>,
}

impl Machine {
    pub fn framebuffers<R: RenderApi>(&self) -> &Vec<ComponentFramebuffer<R>> {
        self.framebuffers
            .get()
            .unwrap()
            .downcast_ref::<Vec<ComponentFramebuffer<R>>>()
            .unwrap()
    }

    pub fn audio_data_callbacks<S: Sample>(&self) -> &Vec<Box<dyn AudioDataCallback<S>>> {
        self.audio_data_callbacks
            .downcast_ref::<Vec<Box<dyn AudioDataCallback<S>>>>()
            .unwrap()
    }
}

pub trait MachineFactory<R: RenderApi, S: Sample>: Debug + Send + Sync + 'static {
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
    ) -> MachineBuilder<R, S>;
}
