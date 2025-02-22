use std::sync::Arc;
use vulkano::image::Image;
use vulkano::memory::allocator::StandardMemoryAllocator;

include!(concat!(env!("OUT_DIR"), "/egui.rs"));

pub struct VulkanEguiRenderer {}

impl VulkanEguiRenderer {
    pub fn new(memory_allocator: Arc<StandardMemoryAllocator>) -> Self {
        todo!()
    }

    pub fn render(
        &mut self,
        context: &egui::Context,
        render_buffer: Arc<Image>,
        full_output: egui::FullOutput,
    ) {
    }
}
