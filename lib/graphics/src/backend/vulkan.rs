use super::ContextExtensionSpecification;
use crate::{
    GraphicsApi,
    shader::{ShaderCache, SpirvShader},
};
use alloc::{sync::Arc, vec::Vec};
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, DeviceExtensions, Queue},
    image::Image,
    memory::allocator::StandardMemoryAllocator,
};

#[derive(Default, Debug)]
/// Marker train for vulkan rendering, supported on major desktop platforms
pub struct Vulkan;

#[derive(Debug, Default, Clone)]
/// Vulkan context extension specification
pub struct VulkanContextExtensionSpecification {
    /// Extensions the vulkan device need to be created with
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

impl GraphicsApi for Vulkan {
    type ComponentGraphicsInitializationData = VulkanComponentInitializationData;
    type ComponentFramebuffer = Arc<Image>;
    type ContextExtensionSpecification = VulkanContextExtensionSpecification;
}

#[derive(Debug)]
/// Initialization data for components that want to do vulkan rendering
pub struct VulkanComponentInitializationData {
    /// The device
    pub device: Arc<Device>,
    /// Memory allocator the rest of the emulator is using
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    /// All the queues found
    pub queues: Vec<Arc<Queue>>,
    /// Command buffer allocator the rest of the emulator is using
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    /// Shader cache for spirv
    pub shader_cache: ShaderCache<SpirvShader>,
}

impl VulkanComponentInitializationData {
    /// Grab the least used queue
    pub fn best_queue(&self) -> Arc<Queue> {
        // FIXME: Naive
        self.queues
            .iter()
            .min_by_key(|q| Arc::strong_count(q))
            .cloned()
            .unwrap()
    }
}
