use crate::{
    environment::Environment, graphics::GraphicsApi, machine::Machine, platform::Platform,
};
use egui::FullOutput;
use nalgebra::Vector2;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

pub trait DisplayApiHandle: HasWindowHandle + HasDisplayHandle + Clone + Debug + 'static {
    fn dimensions(&self) -> Vector2<u32>;
}

/// Extension trait for graphics apis
pub trait GraphicsRuntime<P: Platform>: Debug + Sized + 'static {
    type DisplayApiHandle: DisplayApiHandle;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        required_features: <P::GraphicsApi as GraphicsApi>::Features,
        preferred_features: <P::GraphicsApi as GraphicsApi>::Features,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>>;

    fn component_initialization_data(&self) -> <P::GraphicsApi as GraphicsApi>::InitializationData;

    fn redraw(&mut self, machine: &Machine<P>);

    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: FullOutput);

    fn display_resized(&mut self) {}
}
