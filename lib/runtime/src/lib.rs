use crate::{audio::AudioCallback, component::store::ComponentStore, graphics::GraphicsCallback};
use builder::MachineBuilder;
use input::{VirtualGamepadId, virtual_gamepad::VirtualGamepad};
use memory::memory_translation_table::MemoryTranslationTable;
use multiemu_audio::Sample;
use multiemu_graphics::{GraphicsApi, Software};
use multiemu_rom::{id::RomId, manager::RomManager, system::GameSystem};
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;
use scheduler::Scheduler;
use std::{any::Any, collections::HashMap, fmt::Debug, sync::Arc, vec::Vec};
use utils::Fragile;

pub mod audio;
pub mod builder;
pub mod component;
pub mod graphics;
pub mod input;
pub mod memory;
pub mod processor;
pub mod scheduler;
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
    audio_callbacks: Box<dyn Any + Send + Sync>,
    graphics_callbacks: Fragile<Box<dyn Any>>,
    // Keep the store from dropping
    _component_store: Arc<ComponentStore>,
}

impl Machine {
    pub fn graphics_callbacks<R: GraphicsApi>(&self) -> &[Box<dyn GraphicsCallback<R>>] {
        self.graphics_callbacks
            .get()
            .unwrap()
            .downcast_ref::<Vec<Box<dyn GraphicsCallback<R>>>>()
            .unwrap()
    }

    pub fn audio_callbacks<S: Sample>(&self) -> &[Box<dyn AudioCallback<S>>] {
        self.audio_callbacks
            .downcast_ref::<Vec<Box<dyn AudioCallback<S>>>>()
            .unwrap()
    }
}

pub trait MachineFactory<R: GraphicsApi = Software, S: Sample = f32>:
    Debug + Send + Sync + 'static
{
    fn construct(
        &self,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        sample_rate: Ratio<u32>,
    ) -> MachineBuilder<R, S>;
}
