use egui::FullOutput;
use multiemu_config::Environment;
use multiemu_machine::{
    Machine,
    display::{backend::RenderBackend, shader::ShaderCache},
};
use std::sync::{Arc, RwLock};

/// A backend for a given render backend
pub trait RenderingBackendState: Sized {
    type RenderBackend: RenderBackend;
    type DisplayApiHandle: Send + Sync + Clone + 'static;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        environment: Arc<RwLock<Environment>>,
        shader_cache: Arc<ShaderCache>,
        preferred_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        required_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
    ) -> Result<Self, Box<dyn std::error::Error>>;
    fn component_initialization_data(
        &self,
    ) -> Arc<<Self::RenderBackend as RenderBackend>::ComponentInitializationData>;
    fn redraw(&mut self, machine: &Machine<Self::RenderBackend>);
    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: FullOutput);
    fn surface_resized(&mut self) {}
}
