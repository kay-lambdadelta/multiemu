use crate::rendering_backend::RenderingBackendState;
use create::{create_vulkan_instance, create_vulkan_swapchain, select_vulkan_device};
use multiemu_config::Environment;
use multiemu_machine::component::ComponentId;
use multiemu_machine::display::vulkan::VulkanComponentInitializationData;
use multiemu_machine::display::RenderBackend;
use multiemu_machine::display::vulkan::VulkanRendering;
use nalgebra::Vector2;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, BlitImageInfo,
        CommandBufferUsage,
    },
    device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo},
    image::{sampler::Filter, view::ImageView, Image, ImageLayout},
    instance::Instance,
    memory::allocator::StandardMemoryAllocator,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    single_pass_renderpass,
    swapchain::{
        acquire_next_image, PresentMode, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainPresentInfo,
    },
    sync::GpuFuture,
    Validated, VulkanError, VulkanLibrary,
};
use winit::window::Window;

mod create;
mod gui;

pub struct VulkanRenderingRuntime {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    device: Arc<Device>,
    gui_queue: Arc<Queue>,
    queues_for_components: Vec<Arc<Queue>>,
    swapchain: Arc<Swapchain>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    render_pass: Arc<RenderPass>,
    previous_frame_future: Option<Box<dyn GpuFuture>>,
    framebuffers: Vec<Arc<Framebuffer>>,
    swapchain_images: Vec<Arc<Image>>,
    recreate_swapchain: bool,
    display_api_handle: Arc<Window>,
    component_framebuffers: HashMap<ComponentId, Arc<Image>>,
    environment: Arc<RwLock<Environment>>,
}

impl RenderingBackendState for VulkanRenderingRuntime {
    type RenderBackend = VulkanRendering;
    type DisplayApiHandle = Arc<Window>;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        preferred_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        required_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window_dimensions = display_api_handle.inner_size();
        let window_dimensions = Vector2::new(window_dimensions.width, window_dimensions.height);

        let environment_guard = environment.read().unwrap();
        let library = VulkanLibrary::new().unwrap();

        tracing::info!("Found vulkan {} implementation", library.api_version());

        let instance = create_vulkan_instance(display_api_handle.clone(), library);
        let surface = Surface::from_window(instance.clone(), display_api_handle.clone()).unwrap();

        let Some((physical_device, enabled_device_extensions, queue_family_index)) =
            select_vulkan_device(
                instance.clone(),
                surface.clone(),
                &preferred_extensions.device_extensions,
                &required_extensions.device_extensions,
            )
        else {
            return Err(format!(
                "Could not find a device that satifies all extensions: {:#?}",
                required_extensions.device_extensions
            )
            .into());
        };

        tracing::info!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: enabled_device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();
        let queues: Vec<_> = queues.collect();

        tracing::info!("Using {} queue(s)", queues.len());

        let (gui_queue, queues_for_components) = if queues.len() == 1 {
            (queues[0].clone(), vec![queues[0].clone()])
        } else {
            let (gui_queue, queues) = queues.split_first().unwrap();
            (gui_queue.clone(), queues.to_vec())
        };

        let (swapchain, swapchain_images) = create_vulkan_swapchain(
            device.clone(),
            surface.clone(),
            window_dimensions,
            environment_guard.graphics_setting.vsync,
        );
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let render_pass = single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap();

        let framebuffers: Vec<Arc<Framebuffer>> = swapchain_images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();

                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect();

        drop(environment_guard);

        Ok(Self {
            previous_frame_future: Some(vulkano::sync::now(device.clone()).boxed()),
            instance,
            surface,
            device,
            gui_queue,
            queues_for_components,
            swapchain,
            memory_allocator,
            command_buffer_allocator,
            render_pass,
            framebuffers,
            swapchain_images,
            recreate_swapchain: false,
            display_api_handle,
            component_framebuffers: HashMap::default(),
            environment,
        })
    }

    fn component_initialization_data(
        &self,
    ) -> Rc<<Self::RenderBackend as RenderBackend>::ComponentInitializationData> {
        Rc::new(VulkanComponentInitializationData {
            device: self.device.clone(),
            queues: self.queues_for_components.clone(),
            memory_allocator: self.memory_allocator.clone(),
            command_buffer_allocator: self.command_buffer_allocator.clone(),
        })
    }

    fn redraw(&mut self) {
        let window_dimensions = self.display_api_handle.inner_size();
        let window_dimensions = Vector2::new(window_dimensions.width, window_dimensions.height);

        let environment_guard = self.environment.read().unwrap();

        // Check if vsync settings disagree
        if (self.swapchain.create_info().present_mode == PresentMode::Immediate)
            ^ environment_guard.graphics_setting.vsync
        {
            self.recreate_swapchain = true;
        }

        // HACK: This only works with a single component
        let component_framebuffer = self
            .component_framebuffers
            .values()
            .next()
            .cloned()
            .unwrap();

        // Skip rendering if impossible window size
        if window_dimensions.min() == 0 {
            return;
        }

        if self.recreate_swapchain {
            tracing::trace!("Recreating swapchain");

            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent: window_dimensions.into(),
                    present_mode: if environment_guard.graphics_setting.vsync {
                        PresentMode::Fifo
                    } else {
                        PresentMode::Immediate
                    },
                    ..self.swapchain.create_info()
                })
                .expect("Failed to recreate swapchain");

            let new_framebuffers = new_images
                .iter()
                .map(|image| {
                    let view = ImageView::new_default(image.clone()).unwrap();
                    Framebuffer::new(
                        self.render_pass.clone(),
                        FramebufferCreateInfo {
                            attachments: vec![view],
                            ..Default::default()
                        },
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();

            self.swapchain = new_swapchain;
            self.swapchain_images = new_images;
            self.framebuffers = new_framebuffers;
            self.recreate_swapchain = false;
        }

        let (image_index, recreate_swapchain, acquire_future) = {
            acquire_next_image(self.swapchain.clone(), None).expect("Failed to acquire next image")
        };
        self.recreate_swapchain |= recreate_swapchain;

        let swapchain_image = self.swapchain_images[image_index as usize].clone();

        let mut command_buffer = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.gui_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        command_buffer
            .blit_image(BlitImageInfo {
                src_image_layout: ImageLayout::TransferSrcOptimal,
                dst_image_layout: ImageLayout::TransferDstOptimal,
                filter: Filter::Nearest,
                ..BlitImageInfo::images(component_framebuffer, swapchain_image.clone())
            })
            .unwrap();

        let command_buffer = command_buffer.build().unwrap();

        // Swap that swapchain very painfully
        match self
            .previous_frame_future
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.gui_queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.gui_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush()
            .map_err(Validated::unwrap)
        {
            Ok(previous_frame_future) => {
                self.previous_frame_future = Some(Box::new(previous_frame_future));
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_future = Some(vulkano::sync::now(self.device.clone()).boxed());
            }
            Err(_) => panic!("Failed to present swapchain image"),
        }

        self.previous_frame_future
            .as_mut()
            .unwrap()
            .cleanup_finished();
    }

    fn redraw_menu(&mut self, _egui_context: &egui::Context, _full_output: egui::FullOutput) {}

    fn surface_resized(&mut self) {
        self.recreate_swapchain = true;
    }
}
