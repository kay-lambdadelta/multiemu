use crate::rendering_backend::RenderingBackendState;
use egui::FullOutput;
use glium::backend::Context;
use multiemu_config::Environment;
use multiemu_machine::display::opengl::{OpenGlComponentInitializationData, OpenGlRendering};
use multiemu_machine::display::RenderBackend;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use winit::window::Window;

pub struct OpenGlRenderingRuntime {
    pub context: Rc<Context>,
}

impl RenderingBackendState for OpenGlRenderingRuntime {
    type RenderBackend = OpenGlRendering;
    type DisplayApiHandle = Arc<Window>;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        preferred_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        required_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn Error>> {
        todo!()
    }

    fn component_initialization_data(
        &self,
    ) -> Rc<<Self::RenderBackend as RenderBackend>::ComponentInitializationData> {
        Rc::new(OpenGlComponentInitializationData {
            context: self.context.clone(),
        })
    }

    fn redraw(&mut self) {
        todo!()
    }

    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: FullOutput) {
        todo!()
    }

    fn surface_resized(&mut self) {
        todo!()
    }
}
