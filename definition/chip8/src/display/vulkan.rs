use super::{LORES, SupportedGraphicsApiChip8Display};
use crate::display::Chip8DisplayBackend;
use multiemu_runtime::graphics::{
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
            memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
            sync::GpuFuture,
        },
    },
};
use nalgebra::{DMatrixView, DMatrixViewMut, Vector2};
use palette::{Srgba, named::BLACK};
use std::sync::Arc;

#[derive(Debug)]
pub struct VulkanState {
    pub staging_buffer: Subbuffer<[Srgba<u8>]>,
    pub queue: Arc<Queue>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub framebuffer: <Vulkan as GraphicsApi>::FramebufferTexture,
    pub current_resolution: Vector2<usize>,
}

impl Chip8DisplayBackend for VulkanState {
    type GraphicsApi = Vulkan;

    fn new(component_initialization_data: InitializationData) -> Self {
        let staging_buffer = Buffer::from_iter(
            component_initialization_data.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS
                    | MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            std::iter::repeat_n(BLACK.into(), LORES.cast().product()),
        )
        .unwrap();

        let framebuffer = Image::new(
            component_initialization_data.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [u32::from(LORES.x), u32::from(LORES.y), 1],
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        VulkanState {
            queue: component_initialization_data.best_queue(),
            command_buffer_allocator: component_initialization_data
                .command_buffer_allocator
                .clone(),
            memory_allocator: component_initialization_data.memory_allocator.clone(),
            staging_buffer,
            framebuffer: framebuffer.clone(),
            current_resolution: LORES.cast(),
        }
    }

    fn resize(&mut self, resolution: Vector2<usize>) {
        let mut staging_buffer_guard = self.staging_buffer.write().unwrap();

        let staging_buffer = DMatrixViewMut::from_slice(
            &mut staging_buffer_guard,
            self.current_resolution.x,
            self.current_resolution.y,
        )
        .resize(resolution.x, resolution.y, BLACK.into());
        drop(staging_buffer_guard);

        self.staging_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS
                    | MemoryTypeFilter::PREFER_HOST,
                ..Default::default()
            },
            staging_buffer.into_iter().copied(),
        )
        .unwrap();

        self.framebuffer = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [resolution.x as u32, resolution.y as u32, 1],
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();
        self.current_resolution = resolution;

        self.commit_staging_buffer();
    }

    fn interact_staging_buffer(&self, callback: impl FnOnce(DMatrixView<'_, Srgba<u8>>)) {
        let staging_buffer_guard = self.staging_buffer.read().unwrap();

        callback(DMatrixView::from_slice(
            &staging_buffer_guard,
            self.current_resolution.x,
            self.current_resolution.y,
        ));
    }

    fn interact_staging_buffer_mut(&mut self, callback: impl FnOnce(DMatrixViewMut<Srgba<u8>>)) {
        let mut staging_buffer_guard = self.staging_buffer.write().unwrap();

        callback(DMatrixViewMut::from_slice(
            &mut staging_buffer_guard,
            self.current_resolution.x,
            self.current_resolution.y,
        ));
    }

    fn commit_staging_buffer(&mut self) {
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
        callback: impl FnOnce(&<Self::GraphicsApi as GraphicsApi>::FramebufferTexture),
    ) {
        callback(&self.framebuffer);
    }
}

impl SupportedGraphicsApiChip8Display for Vulkan {
    type Backend = VulkanState;
}
