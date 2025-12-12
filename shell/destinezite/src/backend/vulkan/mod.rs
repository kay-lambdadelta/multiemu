use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use create::{create_vulkan_instance, create_vulkan_swapchain, select_vulkan_device};
use gui::VulkanEguiRenderer;
use multiemu_frontend::{GraphicsRuntime, WindowingHandle, environment::Environment};
use multiemu_runtime::{
    graphics::{
        GraphicsApi,
        vulkan::{
            InitializationData, Vulkan,
            vulkano::{
                Validated, VulkanError, VulkanLibrary,
                command_buffer::{
                    AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage,
                    allocator::StandardCommandBufferAllocator,
                },
                descriptor_set::allocator::StandardDescriptorSetAllocator,
                device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo},
                image::{Image, ImageLayout, sampler::Filter, view::ImageView},
                memory::{
                    MemoryProperties,
                    allocator::{GenericMemoryAllocatorCreateInfo, StandardMemoryAllocator},
                },
                render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
                single_pass_renderpass,
                swapchain::{
                    PresentMode, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
                    SwapchainPresentInfo, acquire_next_image,
                },
                sync::GpuFuture,
            },
        },
    },
    machine::Machine,
    shader::{ShaderCache, SpirvShader},
};
use nalgebra::Vector2;

use crate::windowing::{DesktopPlatform, WinitWindow};

mod create;
mod gui;

static EXISTING_INSTANCE: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct VulkanGraphicsRuntime {
    device: Arc<Device>,
    gui_queue: Arc<Queue>,
    queues_for_components: Arc<Vec<Arc<Queue>>>,
    swapchain: Arc<Swapchain>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    swapchain_images: Vec<Arc<Image>>,
    recreate_swapchain: bool,
    display_api_handle: WinitWindow,
    gui_renderer: VulkanEguiRenderer,
    shader_cache: ShaderCache<SpirvShader>,
}

impl GraphicsRuntime<DesktopPlatform<Vulkan, Self>> for VulkanGraphicsRuntime {
    type WindowingHandle = WinitWindow;

    fn new(
        display_api_handle: Self::WindowingHandle,
        required_features: <Vulkan as GraphicsApi>::Features,
        preferred_features: <Vulkan as GraphicsApi>::Features,
        environment: &Environment,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if EXISTING_INSTANCE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("Cannot create more than one vulkan runtime at a time".into());
        }

        let window_dimensions = display_api_handle.dimensions();
        let shader_cache: ShaderCache<SpirvShader> = ShaderCache::default();

        let library = VulkanLibrary::new().unwrap();

        tracing::info!("Found vulkan {} implementation", library.api_version());

        let instance = create_vulkan_instance(display_api_handle.clone(), library);
        let surface = Surface::from_window(instance.clone(), display_api_handle.inner()).unwrap();

        let Some((physical_device, enabled_device_extensions, queue_family_index)) =
            select_vulkan_device(
                instance.clone(),
                surface.clone(),
                &preferred_features,
                &required_features,
            )
        else {
            return Err(format!(
                "Could not find a device that satifies all extensions: {:#?}",
                required_features
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
        let queues_for_components = Arc::new(queues_for_components);

        let (swapchain, swapchain_images) = create_vulkan_swapchain(
            device.clone(),
            surface.clone(),
            window_dimensions,
            environment.graphics_setting.vsync,
        );
        let memory_allocator = {
            let MemoryProperties { memory_types, .. } =
                device.physical_device().memory_properties();

            let memory_allocator = StandardMemoryAllocator::new(
                device.clone(),
                GenericMemoryAllocatorCreateInfo {
                    // 64 MiB
                    block_sizes: &vec![64 * 1024 * 1024; memory_types.len()],
                    ..Default::default()
                },
            );

            Arc::new(memory_allocator)
        };

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
                    load_op: Load,
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

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let gui_renderer = VulkanEguiRenderer::new(
            device.clone(),
            memory_allocator.clone(),
            descriptor_set_allocator.clone(),
            &shader_cache,
            render_pass.clone(),
        );

        Ok(Self {
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
            gui_renderer,
            shader_cache,
        })
    }

    fn component_initialization_data(&self) -> <Vulkan as GraphicsApi>::InitializationData {
        InitializationData {
            device: self.device.clone(),
            queues: self.queues_for_components.clone(),
            memory_allocator: self.memory_allocator.clone(),
            command_buffer_allocator: self.command_buffer_allocator.clone(),
            shader_cache: self.shader_cache.clone(),
        }
    }

    fn redraw(
        &mut self,
        egui_context: &egui::Context,
        full_output: egui::FullOutput,
        machine: Option<&Machine>,
        environment: &Environment,
    ) {
        let window_dimensions = self.display_api_handle.dimensions();

        // Skip rendering if impossible window size
        if window_dimensions.min() == 0 {
            return;
        }

        let (image_index, acquire_future, swapchain_image) =
            self.swapchain_routines(window_dimensions, environment);

        let swapchain_image_view = ImageView::new_default(swapchain_image.clone()).unwrap();

        let mut command_buffer = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.gui_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        if let Some(machine) = machine {
            for display_path in machine.displays.iter() {
                machine
                    .interact_dyn_mut(display_path, |component| {
                        let display = component.access_framebuffer(display_path);

                        let display: &<Vulkan as GraphicsApi>::FramebufferTexture =
                            display.downcast_ref().unwrap();

                        command_buffer
                            .blit_image(BlitImageInfo {
                                src_image_layout: ImageLayout::TransferSrcOptimal,
                                dst_image_layout: ImageLayout::TransferDstOptimal,
                                filter: Filter::Nearest,
                                ..BlitImageInfo::images(display.clone(), swapchain_image.clone())
                            })
                            .unwrap();
                    })
                    .unwrap();
            }
        }

        self.gui_renderer.render(
            egui_context,
            swapchain_image_view,
            full_output,
            &mut command_buffer,
        );

        self.display_api_handle.inner().pre_present_notify();

        let command_buffer = command_buffer.build().unwrap();
        // Swap that swapchain
        match acquire_future
            .then_execute(self.gui_queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.gui_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush()
            .map_err(Validated::unwrap)
        {
            Ok(future) => {
                future.wait(None).unwrap();
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
            }
            Err(_) => panic!("Failed to present swapchain image"),
        }
    }

    fn display_resized(&mut self) {
        self.recreate_swapchain = true;
    }
}

impl VulkanGraphicsRuntime {
    fn swapchain_routines(
        &mut self,
        window_dimensions: Vector2<u32>,
        environment: &Environment,
    ) -> (u32, SwapchainAcquireFuture, Arc<Image>) {
        // Check if vsync settings disagree
        if (self.swapchain.create_info().present_mode == PresentMode::Immediate)
            == environment.graphics_setting.vsync
        {
            self.recreate_swapchain = true;
        }

        if self.recreate_swapchain {
            tracing::trace!("Recreating swapchain");

            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent: window_dimensions.into(),
                    present_mode: if environment.graphics_setting.vsync {
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

        (image_index, acquire_future, swapchain_image)
    }
}

impl Drop for VulkanGraphicsRuntime {
    fn drop(&mut self) {
        EXISTING_INSTANCE.store(false, Ordering::SeqCst);
    }
}
