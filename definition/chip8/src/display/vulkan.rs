use super::{CHIP8_DIMENSIONS, SupportedRenderApiChip8Display};
use crate::display::{Chip8DisplayBackend, draw_sprite_common};
use multiemu_machine::{
    component::RuntimeEssentials,
    display::backend::{ComponentFramebuffer, vulkan::VulkanRendering},
};
use nalgebra::{DMatrix, DMatrixViewMut, Point2};
use palette::{Srgb, Srgba};
use std::{ops::DerefMut, sync::Arc};
use vulkano::{
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
};

#[derive(Debug)]
pub struct VulkanState {
    pub staging_buffer: Subbuffer<[Srgba<u8>]>,
    pub queue: Arc<Queue>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub render_image: ComponentFramebuffer<VulkanRendering>,
}

impl Chip8DisplayBackend<VulkanRendering> for VulkanState {
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
            vec![Srgba::new(0, 0, 0, 0xff); CHIP8_DIMENSIONS.cast().product()],
        )
        .unwrap();

        let render_image = ComponentFramebuffer::new(
            Image::new(
                component_initialization_data.memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::R8G8B8A8_SRGB,
                    extent: [CHIP8_DIMENSIONS.x as u32, CHIP8_DIMENSIONS.y as u32, 1],
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

    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        let mut staging_buffer = self.staging_buffer.write().unwrap();
        let staging_buffer = DMatrixViewMut::from_slice(staging_buffer.deref_mut(), 64, 32);

        draw_sprite_common(position, sprite, staging_buffer)
    }

    fn set_mode(&mut self, mode: crate::Chip8Kind) {}

    fn clear_display(&self) {
        let mut staging_buffer = self.staging_buffer.write().unwrap();

        staging_buffer.fill(Srgba::new(0, 0, 0, 255));
    }

    fn save_screen_contents(&self) -> DMatrix<Srgb<u8>> {
        let staging_buffer = self.staging_buffer.read().unwrap();

        DMatrix::from_iterator(
            64,
            32,
            staging_buffer
                .iter()
                .map(|pixel| Srgb::new(pixel.red, pixel.green, pixel.blue)),
        )
    }

    fn load_screen_contents(&self, buffer: DMatrix<Srgb<u8>>) {
        let mut staging_buffer = self.staging_buffer.write().unwrap();

        for (source, destination) in buffer.iter().zip(staging_buffer.iter_mut()) {
            *destination = Srgba::new(source.red, source.green, source.blue, 0xff);
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

impl SupportedRenderApiChip8Display for VulkanRendering {
    type Backend = VulkanState;
}
