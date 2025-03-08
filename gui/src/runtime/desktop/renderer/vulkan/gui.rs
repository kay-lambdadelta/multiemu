use egui::TextureId;
use encase::ShaderSize;
use nalgebra::{DMatrix, DMatrixViewMut, Vector2};
use palette::{LinSrgba, Srgba};
use shader::spirv::{FRAGMENT_SHADER_ENTRY, VERTEX_SHADER, VERTEX_SHADER_ENTRY};
use shader::types::VertexInput;
use std::collections::{HashMap, HashSet};
use std::ops::DerefMut;
use std::sync::Arc;
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, BufferImageCopy, CommandBufferUsage, CopyBufferToImageInfo,
    PrimaryCommandBufferAbstract,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::layout::{
    DescriptorSetLayout, DescriptorSetLayoutCreateFlags, DescriptorSetLayoutCreateInfo,
};
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerCreateInfo};
use vulkano::image::view::{ImageView, ImageViewCreateInfo, ImageViewType};
use vulkano::image::{
    Image, ImageCreateInfo, ImageSubresourceLayers, ImageType, ImageUsage, SampleCount,
};
use vulkano::memory::DeviceAlignment;
use vulkano::memory::allocator::{
    AllocationCreateInfo, DeviceLayout, MemoryTypeFilter, StandardMemoryAllocator,
};
use vulkano::pipeline::cache::{PipelineCache, PipelineCacheCreateInfo};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, ColorBlendAttachmentState, ColorBlendState,
};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{
    VertexBufferDescription, VertexInputAttributeDescription, VertexInputBindingDescription,
    VertexInputRate, VertexInputState,
};
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::Subpass;
use vulkano::shader::{ShaderModule, ShaderModuleCreateInfo};
use vulkano::sync::GpuFuture;
use vulkano::{DeviceSize, single_pass_renderpass};

include!(concat!(env!("OUT_DIR"), "/egui.rs"));

const VERTEX_INDEX_DEVICE_ALIGNMENT: DeviceAlignment = DeviceAlignment::of::<u32>();

pub struct VulkanEguiRenderer {
    textures: HashMap<TextureId, Arc<Image>>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    gui_queue: Arc<Queue>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pipeline: Arc<GraphicsPipeline>,
    sampler: Arc<Sampler>,
    vertex_index_buffer_pool: SubbufferAllocator,
    rebind: bool,
}

impl VulkanEguiRenderer {
    pub fn new(
        device: Arc<Device>,
        gui_queue: Arc<Queue>,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    ) -> Self {
        let render_pass = single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: Format::R8G8B8A8_UNORM,
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
        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        // SAFETY: These shaders are pre validated by the wgsl bindgen so this should be safe
        let vertex_shader = unsafe {
            ShaderModule::new(
                device.clone(),
                ShaderModuleCreateInfo::new(shader::spirv::VERTEX_SHADER),
            )
            .unwrap()
        };

        let fragment_shader = unsafe {
            ShaderModule::new(
                device.clone(),
                ShaderModuleCreateInfo::new(shader::spirv::FRAGMENT_SHADER),
            )
            .unwrap()
        };

        let blend = AttachmentBlend {
            src_color_blend_factor: BlendFactor::One,
            src_alpha_blend_factor: BlendFactor::OneMinusDstAlpha,
            dst_color_blend_factor: BlendFactor::One,
            ..AttachmentBlend::alpha()
        };

        let blend_state = ColorBlendState {
            attachments: vec![ColorBlendAttachmentState {
                blend: Some(blend),
                ..Default::default()
            }],
            ..ColorBlendState::default()
        };

        let vertex_index_buffer_pool = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::TRANSFER_SRC
                    | BufferUsage::VERTEX_BUFFER
                    | BufferUsage::INDEX_BUFFER,
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                ..Default::default()
            },
        )
        .unwrap();

        let vertex_shader_entry = vertex_shader.entry_point(VERTEX_SHADER_ENTRY).unwrap();
        let fragment_shader_entry = fragment_shader.entry_point(FRAGMENT_SHADER_ENTRY).unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vertex_shader_entry),
            PipelineShaderStageCreateInfo::new(fragment_shader_entry),
        ];

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        let pipeline = GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: None,
                multisample_state: Some(MultisampleState {
                    rasterization_samples: subpass.num_samples().unwrap_or(SampleCount::Sample1),
                    ..Default::default()
                }),
                color_blend_state: Some(blend_state),
                dynamic_state: HashSet::from_iter([DynamicState::Viewport, DynamicState::Scissor]),
                subpass: Some(subpass.into()),
                rasterization_state: Some(RasterizationState::default()),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap();

        Self {
            textures: HashMap::new(),
            memory_allocator,
            gui_queue,
            command_buffer_allocator,
            descriptor_set_allocator,
            vertex_index_buffer_pool,
            pipeline,
            sampler,
            rebind: true,
        }
    }

    pub fn render(
        &mut self,
        context: &egui::Context,
        render_buffer: Arc<ImageView>,
        full_output: egui::FullOutput,
    ) {
        for remove_texture_id in full_output.textures_delta.free {
            tracing::trace!("Freeing egui texture {:?}", remove_texture_id);
            self.textures.remove(&remove_texture_id);
        }

        let mut command_buffer = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.gui_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        for (new_texture_id, new_texture) in full_output.textures_delta.set {
            tracing::debug!("Adding new egui texture {:?}", new_texture_id);

            if new_texture.pos.is_some() && !self.textures.contains_key(&new_texture_id) {
                panic!("Texture not found: {:?}", new_texture_id);
            }

            let texture_dimensions = Vector2::from(new_texture.image.size());

            let destination_texture = self.textures.entry(new_texture_id).or_insert_with(|| {
                Image::new(
                    self.memory_allocator.clone(),
                    ImageCreateInfo {
                        image_type: ImageType::Dim2d,
                        format: Format::R8G8B8A8_SRGB,
                        extent: [texture_dimensions.x as u32, texture_dimensions.y as u32, 1],
                        usage: ImageUsage::TRANSFER_SRC
                            | ImageUsage::TRANSFER_DST
                            | ImageUsage::SAMPLED,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        ..Default::default()
                    },
                )
                .unwrap()
            });

            let texture_staging_buffer: Subbuffer<[Srgba<u8>]> = match &new_texture.image {
                egui::ImageData::Color(image) => {
                    let image_converter = image
                        .pixels
                        .clone()
                        .into_iter()
                        .map(|pixel| Srgba::from_components(pixel.to_tuple()));

                    Buffer::from_iter(
                        self.memory_allocator.clone(),
                        BufferCreateInfo {
                            usage: BufferUsage::TRANSFER_SRC,
                            ..Default::default()
                        },
                        AllocationCreateInfo {
                            memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                            ..Default::default()
                        },
                        image_converter,
                    )
                    .expect("Failed to create staging buffer")
                }
                egui::ImageData::Font(font_image) => {
                    let image_converter = font_image.pixels.clone().into_iter().map(|coverage| {
                        Srgba::from_linear(LinSrgba::new(coverage, coverage, coverage, coverage))
                    });

                    Buffer::from_iter(
                        self.memory_allocator.clone(),
                        BufferCreateInfo {
                            usage: BufferUsage::TRANSFER_SRC,
                            ..Default::default()
                        },
                        AllocationCreateInfo {
                            memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                            ..Default::default()
                        },
                        image_converter,
                    )
                    .expect("Failed to create staging buffer")
                }
            };

            let texture_update_offset = Vector2::from(new_texture.pos.unwrap_or([0, 0])).cast();

            command_buffer
                .copy_buffer_to_image(CopyBufferToImageInfo {
                    regions: [BufferImageCopy {
                        image_offset: [texture_update_offset.x, texture_update_offset.y, 0],
                        image_extent: [texture_dimensions.x as u32, texture_dimensions.y as u32, 1],
                        image_subresource: ImageSubresourceLayers::from_parameters(
                            destination_texture.format(),
                            1,
                        ),
                        ..Default::default()
                    }]
                    .into(),
                    ..CopyBufferToImageInfo::buffer_image(
                        texture_staging_buffer,
                        destination_texture.clone(),
                    )
                })
                .unwrap();
        }

        if self.rebind {
            self.rebind = false;

            let descriptor_set_layout = self.pipeline.layout().set_layouts()[1].clone();

            command_buffer
                .bind_pipeline_graphics(self.pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    0,
                    DescriptorSet::new(
                        self.descriptor_set_allocator.clone(),
                        descriptor_set_layout,
                        [
                            WriteDescriptorSet::sampler(0, self.sampler.clone()),
                            WriteDescriptorSet::image_view(1, render_buffer.clone()),
                        ],
                        [],
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let layout = DeviceLayout::new(u32::SHADER_SIZE, VERTEX_INDEX_DEVICE_ALIGNMENT).unwrap();
        let vertex_index_buffer: Subbuffer<[u32]> = self
            .vertex_index_buffer_pool
            .allocate_slice(u32::SHADER_SIZE.get())
            .unwrap();

        command_buffer
            .build()
            .unwrap()
            .execute(self.gui_queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();
    }
}
