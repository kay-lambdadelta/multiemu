use crate::{
    GraphicsApi,
    shader::{ShaderCache, SpirvShader},
};
use std::{
    mem::transmute,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use vulkano::{
    buffer::{BufferContents, BufferReadGuard, BufferWriteGuard, Subbuffer},
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, DeviceExtensions, Queue},
    image::Image,
    memory::allocator::StandardMemoryAllocator,
    sync::HostAccessError,
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

pub trait SubbufferExt<T: ?Sized + 'static> {
    fn owned_read(&self) -> Result<OwnedBufferReadGuard<T>, HostAccessError>;
    fn owned_write(&self) -> Result<OwnedBufferWriteGuard<T>, HostAccessError>;
}

impl<T: ?Sized + BufferContents + 'static> SubbufferExt<T> for Subbuffer<T> {
    fn owned_read(&self) -> Result<OwnedBufferReadGuard<T>, HostAccessError> {
        let buffer = Box::new(self.clone());
        let guard = buffer.read()?;

        let guard: BufferReadGuard<'static, T> = unsafe { transmute(guard) };

        Ok(OwnedBufferReadGuard {
            _buffer: buffer,
            guard: Some(guard),
        })
    }

    fn owned_write(&self) -> Result<OwnedBufferWriteGuard<T>, HostAccessError> {
        let buffer = Box::new(self.clone());
        let guard = buffer.write()?;

        let guard: BufferWriteGuard<'static, T> = unsafe { transmute(guard) };

        Ok(OwnedBufferWriteGuard {
            _buffer: buffer,
            guard: Some(guard),
        })
    }
}

#[derive(Debug)]
pub struct OwnedBufferReadGuard<T: ?Sized + 'static> {
    _buffer: Box<Subbuffer<T>>,
    guard: Option<BufferReadGuard<'static, T>>,
}

impl<T: ?Sized + 'static> Drop for OwnedBufferReadGuard<T> {
    fn drop(&mut self) {
        self.guard.take();
    }
}

impl<T: ?Sized + 'static> Deref for OwnedBufferReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.as_deref().unwrap()
    }
}

#[derive(Debug)]
pub struct OwnedBufferWriteGuard<T: ?Sized + 'static> {
    _buffer: Box<Subbuffer<T>>,
    guard: Option<BufferWriteGuard<'static, T>>,
}

impl<T: ?Sized + 'static> Drop for OwnedBufferWriteGuard<T> {
    fn drop(&mut self) {
        self.guard.take();
    }
}

impl<T: ?Sized + 'static> Deref for OwnedBufferWriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.as_deref().unwrap()
    }
}

impl<T: ?Sized + 'static> DerefMut for OwnedBufferWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_deref_mut().unwrap()
    }
}
