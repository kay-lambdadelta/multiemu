use egui::FullOutput;
use multiemu_runtime::{
    environment::Environment, graphics::GraphicsApi, machine::Machine, platform::Platform,
};
use nalgebra::Vector2;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

/// Handle to the surface we will be rendering graphics to
pub trait DisplayApiHandle: Clone + Debug + 'static {
    /// Get the dimensions
    fn dimensions(&self) -> Vector2<u32>;
}

/// Extension trait for graphics apis
pub trait GraphicsRuntime<P: Platform>: Debug + Sized + 'static {
    /// The type of display api handle that is required here
    type DisplayApiHandle: DisplayApiHandle;

    /// Create the graphics runtime
    fn new(
        display_api_handle: Self::DisplayApiHandle,
        required_features: <P::GraphicsApi as GraphicsApi>::Features,
        preferred_features: <P::GraphicsApi as GraphicsApi>::Features,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>>;

    /// Graphics data components require
    fn component_initialization_data(&self) -> <P::GraphicsApi as GraphicsApi>::InitializationData;

    /// Draw the next frame using this machine
    fn redraw(&mut self, machine: &Machine<P>);

    /// Draw the next frame using the menu
    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: FullOutput);

    /// Notification that the render surface resized
    fn display_resized(&mut self) {}
}
