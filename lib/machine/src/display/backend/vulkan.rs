use super::ContextExtensionSpecification;
use crate::display::RenderApi;
use multiemu_config::graphics::GraphicsApi;
use std::sync::Arc;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, DeviceExtensions, Queue},
    image::Image,
    memory::allocator::StandardMemoryAllocator,
};

#[derive(Default, Debug)]
pub struct VulkanRendering;

#[derive(Debug, Default, Clone)]
pub struct VulkanContextExtensionSpecification {
    pub device_extensions: DeviceExtensions,
}

impl ContextExtensionSpecification for VulkanContextExtensionSpecification {
    fn combine(self, other: Self) -> Self
    where
        Self: Sized,
    {
        Self {
            device_extensions: self.device_extensions | other.device_extensions,
        }
    }
}

impl RenderApi for VulkanRendering {
    const GRAPHICS_API: GraphicsApi = GraphicsApi::Vulkan;
    type ComponentInitializationData = VulkanComponentInitializationData;
    type ComponentFramebufferInner = Image;
    type ContextExtensionSpecification = VulkanContextExtensionSpecification;
}

#[derive(Debug)]
pub struct VulkanComponentInitializationData {
    pub device: Arc<Device>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub queues: Vec<Arc<Queue>>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

impl VulkanComponentInitializationData {
    pub fn best_queue(&self) -> Arc<Queue> {
        // FIXME: Naive
        self.queues
            .iter()
            .min_by_key(|q| Arc::strong_count(q))
            .cloned()
            .unwrap()
    }
}
