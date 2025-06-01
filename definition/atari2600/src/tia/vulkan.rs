use super::{
    FramebufferGuard, SCANLINE_LENGTH, SupportedRenderApiTia, TiaDisplayBackend, region::Region,
};
use multiemu_machine::{
    component::RuntimeEssentials,
    display::backend::{ComponentFramebuffer, vulkan::VulkanRendering},
};
use nalgebra::DMatrixViewMut;
use palette::Srgba;
use std::{marker::PhantomData, sync::Arc};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, BufferWriteGuard, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
        PrimaryCommandBufferAbstract, allocator::StandardCommandBufferAllocator,
    },
    device::Queue,
    format::Format,
    image::{Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::GpuFuture,
};

#[derive(Debug)]
pub struct VulkanState {
    pub staging_buffer: Subbuffer<[Srgba<u8>]>,
    pub queue: Arc<Queue>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub render_image: ComponentFramebuffer<VulkanRendering>,
}

struct VulkanFramebufferGuard<'a, R: Region> {
    staging_buffer_guard: BufferWriteGuard<'a, [Srgba<u8>]>,
    _phantom: PhantomData<R>,
}

impl<R: Region> FramebufferGuard for VulkanFramebufferGuard<'_, R> {
    fn get(&mut self) -> DMatrixViewMut<'_, Srgba<u8>> {
        DMatrixViewMut::from_slice(
            &mut self.staging_buffer_guard,
            SCANLINE_LENGTH as usize,
            R::TOTAL_SCANLINES as usize,
        )
    }
}

impl<R: Region> TiaDisplayBackend<R, VulkanRendering> for VulkanState {
    fn new(
        essentials: &RuntimeEssentials<VulkanRendering>,
    ) -> (Self, ComponentFramebuffer<VulkanRendering>) {
        let component_initialization_data = essentials.render_initialization_data.get().unwrap();

        let staging_buffer = Buffer::from_iter(
            component_initialization_data.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            std::iter::repeat_n(
                Srgba::new(0, 0, 0, 0xff),
                SCANLINE_LENGTH as usize * R::TOTAL_SCANLINES as usize,
            ),
        )
        .unwrap();

        let render_image = ComponentFramebuffer::new(
            Image::new(
                component_initialization_data.memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::R8G8B8A8_SRGB,
                    extent: [SCANLINE_LENGTH as u32, R::TOTAL_SCANLINES as u32, 1],
                    usage: ImageUsage::TRANSFER_SRC
                        | ImageUsage::TRANSFER_DST
                        | ImageUsage::SAMPLED,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .unwrap(),
        );

        (
            VulkanState {
                queue: component_initialization_data.best_queue(),
                command_buffer_allocator: component_initialization_data
                    .command_buffer_allocator
                    .clone(),
                staging_buffer,
                render_image: render_image.clone(),
            },
            render_image,
        )
    }

    fn lock_framebuffer(&self) -> impl FramebufferGuard {
        let staging_buffer_guard = self.staging_buffer.write().unwrap();

        VulkanFramebufferGuard::<R> {
            staging_buffer_guard,
            _phantom: PhantomData,
        }
    }

    fn commit_display(&self) {
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
                self.render_image.load(),
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
}

impl SupportedRenderApiTia for VulkanRendering {
    type Backend<R: Region> = VulkanState;
}
