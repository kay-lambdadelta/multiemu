use egui::FullOutput;
use multiemu_config::Environment;
use multiemu_runtime::{
    Machine,
    display::{RenderExtensions, backend::RenderApi},
};
use nalgebra::Vector2;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

pub trait DisplayApiHandle: Send + Sync + Clone + Debug + 'static {
    fn dimensions(&self) -> Vector2<u32>;
}

/// A backend for a given render backend
pub trait RenderingBackendState: Debug + Sized {
    type RenderApi: RenderApi;
    type DisplayApiHandle: DisplayApiHandle;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        render_extensions: RenderExtensions<Self::RenderApi>,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>>;
    fn component_initialization_data(
        &self,
    ) -> <Self::RenderApi as RenderApi>::ComponentInitializationData;
    fn redraw(&mut self, machine: &Machine);
    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: FullOutput);
    fn surface_resized(&mut self) {}
}
