use super::{SupportedGraphicsApiTia, TiaDisplayBackend};
use crate::tia::{SCANLINE_LENGTH, region::Region};
use multiemu_graphics::{
    GraphicsApi,
    vulkan::{
        InitializationData, Vulkan,
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
use palette::Srgba;
use std::sync::Arc;

#[derive(Debug)]
pub struct VulkanState {
    pub staging_buffer: Subbuffer<[Srgba<u8>]>,
    pub queue: Arc<Queue>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub framebuffer: Arc<Image>,
}

impl<R: Region> TiaDisplayBackend<R> for VulkanState {
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
                Srgba::new(0, 0, 0, 0xff),
                SCANLINE_LENGTH as usize * R::TOTAL_SCANLINES as usize,
            ),
        )
        .unwrap();

        let framebuffer = Image::new(
            initialization_data.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [SCANLINE_LENGTH as u32, R::TOTAL_SCANLINES as u32, 1],
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
            framebuffer: framebuffer.clone(),
        }
    }

    fn modify_staging_buffer(&self, callback: impl FnOnce(DMatrixViewMut<'_, Srgba<u8>>)) {
        let mut staging_buffer_guard = self.staging_buffer.write().unwrap();

        callback(DMatrixViewMut::from_slice(
            &mut staging_buffer_guard,
            SCANLINE_LENGTH as usize,
            R::TOTAL_SCANLINES as usize,
        ));
    }

    fn commit_staging_buffer(&self) {
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
        &self,
        callback: impl FnOnce(&<Vulkan as GraphicsApi>::FramebufferTexture),
    ) {
        callback(&self.framebuffer);
    }
}

impl SupportedGraphicsApiTia for Vulkan {
    type Backend<R: Region> = VulkanState;
}
