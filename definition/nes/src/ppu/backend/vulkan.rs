use super::{PpuDisplayBackend, SupportedGraphicsApiPpu};
use crate::ppu::{VISIBLE_SCANLINE_LENGTH, region::Region};
use multiemu_base::graphics::{
    GraphicsApi,
    vulkan::{
        InitializationData, OwnedBufferWriteGuard, SubbufferExt, Vulkan,
        vulkano::{
            buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
            command_buffer::{
                AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
                PrimaryCommandBufferAbstract, allocator::StandardCommandBufferAllocator,
            },
            device::Queue,
            format::Format,
            image::{Image, ImageCreateInfo, ImageType, ImageUsage},
            memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
            sync::GpuFuture,
        },
    },
};
use nalgebra::DMatrixViewMut;
use palette::{Srgba, named::BLACK};
use std::sync::Arc;

#[derive(Debug)]
pub struct VulkanState {
    pub staging_buffer: Subbuffer<[Srgba<u8>]>,
    pub staging_buffer_guard: Option<OwnedBufferWriteGuard<[Srgba<u8>]>>,
    pub queue: Arc<Queue>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub framebuffer: Arc<Image>,
}

impl<R: Region> PpuDisplayBackend<R> for VulkanState {
    type GraphicsApi = Vulkan;

    fn new(initialization_data: InitializationData) -> Self {
        let staging_buffer = Buffer::from_iter(
            initialization_data.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS
                    | MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            std::iter::repeat_n(
                BLACK.into(),
                VISIBLE_SCANLINE_LENGTH as usize * R::VISIBLE_SCANLINES as usize,
            ),
        )
        .unwrap();

        let framebuffer = Image::new(
            initialization_data.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [
                    VISIBLE_SCANLINE_LENGTH as u32,
                    R::VISIBLE_SCANLINES as u32,
                    1,
                ],
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        VulkanState {
            queue: initialization_data.best_queue(),
            command_buffer_allocator: initialization_data.command_buffer_allocator.clone(),
            staging_buffer,
            staging_buffer_guard: None,
            framebuffer: framebuffer.clone(),
        }
    }

    fn modify_staging_buffer(&mut self, callback: impl FnOnce(DMatrixViewMut<'_, Srgba<u8>>)) {
        let staging_buffer_guard = self
            .staging_buffer_guard
            .get_or_insert_with(|| self.staging_buffer.owned_write().unwrap());

        callback(DMatrixViewMut::from_slice(
            staging_buffer_guard,
            VISIBLE_SCANLINE_LENGTH as usize,
            R::VISIBLE_SCANLINES as usize,
        ));
    }

    fn commit_staging_buffer(&mut self) {
        // Drop the owned guard
        self.staging_buffer_guard.take();

        let mut command_buffer = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        command_buffer
            // Copy the staging buffer to the image
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                self.staging_buffer.clone(),
                self.framebuffer.clone(),
            ))
            .unwrap();

        command_buffer
            .build()
            .unwrap()
            .execute(self.queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();
    }

    fn access_framebuffer(
        &mut self,
        callback: impl FnOnce(&<Vulkan as GraphicsApi>::FramebufferTexture),
    ) {
        callback(&self.framebuffer);
    }
}

impl SupportedGraphicsApiPpu for Vulkan {
    type Backend<R: Region> = VulkanState;
}
