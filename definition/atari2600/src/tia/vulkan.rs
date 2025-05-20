use super::{SCANLINE_LENGTH, State, SupportedRenderApiTia, TiaDisplayBackend, region::Region};
use crate::tia::{__seal_supported_render_api_tia, __seal_tia_display_backend};
use multiemu_machine::{
    component::RuntimeEssentials,
    display::backend::{ComponentFramebuffer, vulkan::VulkanRendering},
};
use nalgebra::{DMatrix, DMatrixViewMut, Point2};
use palette::{Srgb, Srgba};
use sealed::sealed;
use std::sync::Arc;
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

#[sealed]
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

    fn draw(&self, state: &State, position: Point2<u16>, hue: u8, luminosity: u8) {
        let real_color = R::color_to_srgb(hue, luminosity);

        let mut staging_buffer = self.staging_buffer.write().unwrap();

        let mut staging_buffer_view = DMatrixViewMut::from_slice(
            staging_buffer.as_mut(),
            SCANLINE_LENGTH as usize,
            R::TOTAL_SCANLINES as usize,
        );

        let color = Srgba::new(real_color.red, real_color.green, real_color.blue, 0xff);
        staging_buffer_view[(position.x as usize, position.y as usize)] = color;
    }

    fn save_screen_contents(&self) -> DMatrix<Srgb<u8>> {
        let staging_buffer = self.staging_buffer.read().unwrap();

        DMatrix::from_row_iterator(
            SCANLINE_LENGTH as usize,
            R::TOTAL_SCANLINES as usize,
            staging_buffer
                .iter()
                .map(|color| Srgb::new(color.red, color.green, color.blue)),
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

#[sealed]
impl SupportedRenderApiTia for VulkanRendering {
    type Backend<R: Region> = VulkanState;
}
