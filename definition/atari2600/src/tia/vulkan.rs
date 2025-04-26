use super::{SCANLINE_LENGTH, Tia, TiaDisplayBackend, region::Region};
use arc_swap::ArcSwap;
use multiemu_machine::display::backend::{
    RenderBackend,
    vulkan::{VulkanComponentFramebuffer, VulkanRendering},
};
use nalgebra::{DMatrix, DMatrixViewMut, Point2};
use palette::{Srgb, Srgba};
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
    pub render_image: Arc<ArcSwap<Image>>,
}

impl<R: Region> TiaDisplayBackend<R> for VulkanState {
    fn draw(&self, position: Point2<u16>, hue: u8, luminosity: u8) {
        let real_color = R::color_to_srgb(hue, luminosity);

        let mut staging_buffer = self.staging_buffer.write().unwrap();

        let mut staging_buffer_view = DMatrixViewMut::from_slice(
            staging_buffer.as_mut(),
            R::TOTAL_SCANLINES as usize,
            SCANLINE_LENGTH as usize,
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
                self.render_image.load_full(),
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

pub fn set_display_data<R: Region>(
    display: &Tia<R>,
    initialization_data: Arc<<VulkanRendering as RenderBackend>::ComponentInitializationData>,
) -> VulkanComponentFramebuffer {
    let staging_buffer = Buffer::from_iter(
        initialization_data.memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
        std::iter::repeat_n(
            Srgba::new(0, 0, 0, 0),
            SCANLINE_LENGTH as usize * R::TOTAL_SCANLINES as usize,
        ),
    )
    .unwrap();

    let render_image = Arc::new(ArcSwap::new(
        Image::new(
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
        .unwrap(),
    ));

    let _ = display.display_backend.set(Box::new(VulkanState {
        queue: initialization_data.best_queue(),
        command_buffer_allocator: initialization_data.command_buffer_allocator.clone(),
        staging_buffer,
        render_image: render_image.clone(),
    }));

    render_image
}
