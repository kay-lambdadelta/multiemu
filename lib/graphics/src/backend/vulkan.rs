use crate::{
    GraphicsApi,
    shader::{ShaderCache, SpirvShader},
};
use std::sync::Arc;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, DeviceExtensions, Queue},
    image::Image,
    memory::allocator::StandardMemoryAllocator,
};

pub use vulkano;

#[derive(Default, Debug)]
/// Marker train for vulkan rendering, supported on major desktop platforms
pub struct Vulkan;

pub type VulkanFeatures = DeviceExtensions;
pub type FramebufferTexture = Arc<Image>;

impl GraphicsApi for Vulkan {
    type InitializationData = InitializationData;
    type FramebufferTexture = FramebufferTexture;
    type Features = VulkanFeatures;
}

#[derive(Debug, Clone)]
/// Initialization data for components that want to do vulkan rendering
pub struct InitializationData {
    /// The device
    pub device: Arc<Device>,
    /// Memory allocator the rest of the emulator is using
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    /// All the queues found
    pub queues: Arc<Vec<Arc<Queue>>>,
    /// Command buffer allocator the rest of the emulator is using
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    /// Shader cache for spirv
    pub shader_cache: ShaderCache<SpirvShader>,
}

impl InitializationData {
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
