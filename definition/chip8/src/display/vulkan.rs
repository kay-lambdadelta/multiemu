use crate::display::{Chip8Display, Chip8DisplayBackend, draw_sprite_common};
use crossbeam::channel::Sender;
use multiemu_machine::display::RenderBackend;
use multiemu_machine::display::vulkan::VulkanRendering;
use nalgebra::{DMatrix, DMatrixViewMut, Point2};
use palette::Srgba;
use std::cell::RefCell;
use std::{ops::DerefMut, sync::Arc};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::format::Format;
use vulkano::image::{ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
        PrimaryCommandBufferAbstract, allocator::StandardCommandBufferAllocator,
    },
    device::Queue,
    image::Image,
    sync::GpuFuture,
};

#[derive(Debug)]
pub struct VulkanState {
    pub staging_buffer: Subbuffer<[Srgba<u8>]>,
    pub queue: Arc<Queue>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub render_image: RefCell<Arc<Image>>,
    pub frame_sender: Sender<Arc<Image>>,
}

impl Chip8DisplayBackend for VulkanState {
    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        let mut staging_buffer = self.staging_buffer.write().unwrap();
        let staging_buffer = DMatrixViewMut::from_slice(staging_buffer.deref_mut(), 64, 32);

        draw_sprite_common(position, sprite, staging_buffer)
    }

    fn clear_display(&self) {
        let mut staging_buffer = self.staging_buffer.write().unwrap();

        staging_buffer.fill(Srgba::new(0, 0, 0, 255));
    }

    fn save_screen_contents(&self) -> DMatrix<Srgba<u8>> {
        let staging_buffer = self.staging_buffer.read().unwrap();

        DMatrix::from_vec(64, 32, staging_buffer.to_vec())
    }

    fn load_screen_contents(&self, buffer: DMatrix<Srgba<u8>>) {
        let mut staging_buffer = self.staging_buffer.write().unwrap();

        staging_buffer.copy_from_slice(buffer.as_slice());
    }

    fn commit_display(&self) {
        if self.frame_sender.is_full() {
            return;
        }

        let render_image = self.render_image.borrow().clone();

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
                render_image.clone(),
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

        let _ = self.frame_sender.try_send(render_image);
    }
}

pub fn set_display_data(
    display: &Chip8Display,
    initialization_data: Arc<<VulkanRendering as RenderBackend>::ComponentInitializationData>,
    frame_sender: Sender<<VulkanRendering as RenderBackend>::ComponentFramebuffer>,
) {
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
        vec![Srgba::new(0, 0, 0, 0xff); 64 * 32],
    )
    .unwrap();

    let render_image = RefCell::new(
        Image::new(
            initialization_data.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_SRGB,
                extent: [64, 32, 1],
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    );

    let _ = display.state.set(Box::new(VulkanState {
        queue: initialization_data.best_queue(),
        command_buffer_allocator: initialization_data.command_buffer_allocator.clone(),
        staging_buffer,
        render_image: render_image.clone(),
        frame_sender,
    }));
}
