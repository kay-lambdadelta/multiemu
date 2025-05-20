use egui::FullOutput;
use multiemu_config::Environment;
use multiemu_machine::{
    Machine,
    display::{RenderExtensions, backend::RenderApi, shader::ShaderCache},
};
use nalgebra::Vector2;
use std::sync::{Arc, RwLock};

pub trait DisplayApiHandle: Send + Sync + Clone + 'static {
    fn dimensions(&self) -> Vector2<u16>;
}

/// A backend for a given render backend
pub trait RenderingBackendState: Sized {
    type RenderApi: RenderApi;
    type DisplayApiHandle: DisplayApiHandle;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
        render_extensions: RenderExtensions<Self::RenderApi>,
    ) -> Result<Self, Box<dyn std::error::Error>>;
    fn component_initialization_data(
        &self,
    ) -> <Self::RenderApi as RenderApi>::ComponentInitializationData;
    fn redraw(&mut self, machine: &Machine<Self::RenderApi>);
    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: FullOutput);
    fn surface_resized(&mut self) {}
}
