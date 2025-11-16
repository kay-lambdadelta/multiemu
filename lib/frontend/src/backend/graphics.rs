use std::fmt::Debug;

use egui::FullOutput;
use multiemu_runtime::{graphics::GraphicsApi, machine::Machine, platform::Platform};
use nalgebra::Vector2;

use crate::environment::Environment;

/// Handle to the surface we will be rendering graphics to
pub trait WindowingHandle: Clone + Debug + 'static {
    /// Get the dimensions
    fn dimensions(&self) -> Vector2<u32>;
}

/// Extension trait for graphics apis
pub trait GraphicsRuntime<P: Platform>: Debug + Sized + 'static {
    /// The type of display api handle that is required here
    type WindowingHandle: WindowingHandle;

    /// Create the graphics runtime
    fn new(
        display_api_handle: Self::WindowingHandle,
        required_features: <P::GraphicsApi as GraphicsApi>::Features,
        preferred_features: <P::GraphicsApi as GraphicsApi>::Features,
        environment: &Environment,
    ) -> Result<Self, Box<dyn std::error::Error>>;

    /// Graphics data components require
    fn component_initialization_data(&self) -> <P::GraphicsApi as GraphicsApi>::InitializationData;

    /// Draw the next frame
    fn redraw(
        &mut self,
        egui_context: &egui::Context,
        full_output: FullOutput,
        machine: Option<&Machine<P>>,
        environment: &Environment,
    );

    /// Notification that the render surface resized
    fn display_resized(&mut self) {}
}
