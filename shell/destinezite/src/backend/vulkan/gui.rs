use std::{
    collections::{HashMap, HashSet},
    mem::offset_of,
    num::NonZero,
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};
use egui::{TextureId, epaint::Primitive};
use multiemu_frontend::EGUI_WGSL_SHADER;
use multiemu_runtime::{
    graphics::{
        GraphicsVersion,
        vulkan::vulkano::{
            DeviceSize,
            buffer::{
                Buffer, BufferCreateInfo, BufferUsage, Subbuffer,
                allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
            },
            command_buffer::{
                AutoCommandBufferBuilder, BufferImageCopy, CopyBufferToImageInfo,
                PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
                SubpassEndInfo,
            },
            descriptor_set::{
                DescriptorSet, WriteDescriptorSet, allocator::StandardDescriptorSetAllocator,
            },
            device::Device,
            format::Format,
            image::{
                Image, ImageCreateInfo, ImageSubresourceLayers, ImageType, ImageUsage, SampleCount,
                sampler::{
                    Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode,
                },
                view::{ImageView, ImageViewCreateInfo},
            },
            memory::{
                DeviceAlignment,
                allocator::{
                    AllocationCreateInfo, DeviceLayout, MemoryTypeFilter, StandardMemoryAllocator,
                },
            },
            pipeline::{
                DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
                PipelineShaderStageCreateInfo,
                graphics::{
                    GraphicsPipelineCreateInfo,
                    color_blend::{
                        AttachmentBlend, BlendFactor, ColorBlendAttachmentState, ColorBlendState,
                    },
                    input_assembly::InputAssemblyState,
                    multisample::MultisampleState,
                    rasterization::{CullMode, RasterizationState},
                    vertex_input::{
                        VertexInputAttributeDescription, VertexInputBindingDescription,
                        VertexInputState,
                    },
                    viewport::{Viewport, ViewportState},
                },
                layout::PipelineDescriptorSetLayoutCreateInfo,
            },
            render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
            shader::{ShaderModule, ShaderModuleCreateInfo},
        },
    },
    shader::{ShaderCache, SpirvShader},
};
use nalgebra::{Point2, Vector2};
use palette::Srgba;

#[derive(Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    position: Point2<f32>,
    uv: Point2<f32>,
    color: Srgba<u8>,
}

impl From<egui::epaint::Vertex> for Vertex {
    fn from(vertex: egui::epaint::Vertex) -> Self {
        Vertex {
            position: Point2::new(vertex.pos.x, vertex.pos.y),
            uv: Point2::new(vertex.uv.x, vertex.uv.y),
            color: Srgba::from_components(vertex.color.to_tuple()),
        }
    }
}

const VERTEX_INDEX_DEVICE_ALIGNMENT: DeviceAlignment = DeviceAlignment::of::<u32>();
const VERTEX_DEVICE_ALIGNMENT: DeviceAlignment = DeviceAlignment::of::<Vertex>();

#[derive(Debug)]
pub struct VulkanEguiRenderer {
    /// Stored textures and their descriptor sets
    textures: HashMap<TextureId, (Arc<Image>, Arc<DescriptorSet>)>,
    /// memory allocator
    memory_allocator: Arc<StandardMemoryAllocator>,
    /// descriptor set allocator
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    /// vulkan pipeline
    pipeline: Arc<GraphicsPipeline>,
    vertex_buffer_pool: SubbufferAllocator,
    /// screen size uniform
    screen_size: Subbuffer<Vector2<f32>>,
    render_pass: Arc<RenderPass>,
    screen_size_sampler_descriptor_set: Arc<DescriptorSet>,
}

impl VulkanEguiRenderer {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<StandardMemoryAllocator>,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
        shader_cache: &ShaderCache<SpirvShader>,
        render_pass: Arc<RenderPass>,
    ) -> Self {
        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let shader = shader_cache
            .get(EGUI_WGSL_SHADER, GraphicsVersion { major: 1, minor: 0 })
            .unwrap();

        // These shaders are pre validated by naga so this should be safe
        let vertex_shader = unsafe {
            ShaderModule::new(device.clone(), ShaderModuleCreateInfo::new(&shader.vertex)).unwrap()
        };

        let fragment_shader = unsafe {
            ShaderModule::new(
                device.clone(),
                ShaderModuleCreateInfo::new(shader.fragment.as_slice()),
            )
            .unwrap()
        };

        let blend = AttachmentBlend {
            src_alpha_blend_factor: BlendFactor::One,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            ..AttachmentBlend::alpha()
        };

        let blend_state = ColorBlendState {
            attachments: vec![ColorBlendAttachmentState {
                blend: Some(blend),
                ..Default::default()
            }],
            ..Default::default()
        };

        let vertex_buffer_pool = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::VERTEX_BUFFER | BufferUsage::INDEX_BUFFER,
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                mipmap_mode: SamplerMipmapMode::Linear,
                ..Default::default()
            },
        )
        .unwrap();

        let vertex_shader_entry = vertex_shader.entry_point(&shader.vertex_entry).unwrap();
        let fragment_shader_entry = fragment_shader.entry_point(&shader.fragment_entry).unwrap();

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

        let vertex_input_state = VertexInputState::new()
            .binding(
                0,
                VertexInputBindingDescription {
                    stride: size_of::<Vertex>() as u32,
                    ..Default::default()
                },
            )
            .attribute(
                0,
                VertexInputAttributeDescription {
                    binding: 0,
                    format: Format::R32G32_SFLOAT,
                    offset: offset_of!(Vertex, position) as u32,
                    ..Default::default()
                },
            )
            .attribute(
                1,
                VertexInputAttributeDescription {
                    binding: 0,
                    format: Format::R32G32_SFLOAT,
                    offset: offset_of!(Vertex, uv) as u32,
                    ..Default::default()
                },
            )
            .attribute(
                2,
                VertexInputAttributeDescription {
                    binding: 0,
                    format: Format::R8G8B8A8_UNORM,
                    offset: offset_of!(Vertex, color) as u32,
                    ..Default::default()
                },
            );

        let pipeline = GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                multisample_state: Some(MultisampleState {
                    rasterization_samples: subpass.num_samples().unwrap_or(SampleCount::Sample1),
                    ..Default::default()
                }),
                color_blend_state: Some(blend_state),
                dynamic_state: HashSet::from_iter([DynamicState::Viewport]),
                subpass: Some(subpass.into()),
                rasterization_state: Some(RasterizationState {
                    cull_mode: CullMode::None,
                    ..Default::default()
                }),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap();

        let screen_size = Buffer::from_data(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            Vector2::default(),
        )
        .unwrap();

        let screen_size_sampler_descriptor_set = DescriptorSet::new(
            descriptor_set_allocator.clone(),
            pipeline.layout().set_layouts()[0].clone(),
            [
                WriteDescriptorSet::buffer(0, screen_size.clone()),
                WriteDescriptorSet::sampler(1, sampler.clone()),
            ],
            [],
        )
        .unwrap();

        Self {
            textures: HashMap::new(),
            memory_allocator,
            descriptor_set_allocator,
            vertex_buffer_pool,
            pipeline,
            screen_size,
            render_pass,
            screen_size_sampler_descriptor_set,
        }
    }

    pub fn render(
        &mut self,
        context: &egui::Context,
        render_buffer: Arc<ImageView>,
        full_output: egui::FullOutput,
        command_buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        let render_buffer_dimensions = Vector2::new(
            render_buffer.image().extent()[0],
            render_buffer.image().extent()[1],
        );

        for remove_texture_id in full_output.textures_delta.free {
            tracing::trace!("Freeing egui texture {:?}", remove_texture_id);
            self.textures.remove(&remove_texture_id);
        }

        for (new_texture_id, new_texture) in full_output.textures_delta.set {
            tracing::debug!("Adding new egui texture {:?}", new_texture_id);

            if new_texture.pos.is_some() && !self.textures.contains_key(&new_texture_id) {
                panic!("Texture not found: {:?}", new_texture_id);
            }

            let new_texture_dimensions = Vector2::from(new_texture.image.size());

            let (destination_texture, _) =
                self.textures.entry(new_texture_id).or_insert_with(|| {
                    let texture = Image::new(
                        self.memory_allocator.clone(),
                        ImageCreateInfo {
                            image_type: ImageType::Dim2d,
                            format: Format::R8G8B8A8_SRGB,
                            extent: [
                                new_texture_dimensions.x as u32,
                                new_texture_dimensions.y as u32,
                                1,
                            ],
                            usage: ImageUsage::TRANSFER_SRC
                                | ImageUsage::TRANSFER_DST
                                | ImageUsage::SAMPLED,
                            mip_levels: 1,
                            ..Default::default()
                        },
                        AllocationCreateInfo {
                            ..Default::default()
                        },
                    )
                    .unwrap();

                    let texture_view =
                        ImageView::new(texture.clone(), ImageViewCreateInfo::from_image(&texture))
                            .unwrap();

                    let layout = self.pipeline.layout().set_layouts()[1].clone();

                    let descriptor_set = DescriptorSet::new(
                        self.descriptor_set_allocator.clone(),
                        layout,
                        [WriteDescriptorSet::image_view(0, texture_view.clone())],
                        [],
                    )
                    .unwrap();

                    (texture, descriptor_set)
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
            };

            let texture_update_offset = Vector2::from(new_texture.pos.unwrap_or([0, 0])).cast();

            command_buffer
                .copy_buffer_to_image(CopyBufferToImageInfo {
                    regions: [BufferImageCopy {
                        image_offset: [texture_update_offset.x, texture_update_offset.y, 0],
                        image_extent: [
                            new_texture_dimensions.x as u32,
                            new_texture_dimensions.y as u32,
                            1,
                        ],
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

        let mut screen_size_guard = self.screen_size.write().unwrap();
        *screen_size_guard = render_buffer_dimensions.cast() / full_output.pixels_per_point;
        drop(screen_size_guard);

        let framebuffer = Framebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![render_buffer.clone()],
                ..Default::default()
            },
        )
        .unwrap();

        command_buffer
            .set_viewport(
                0,
                [Viewport {
                    offset: [0.0, 0.0],
                    extent: render_buffer_dimensions.map(|v| v as f32).into(),
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .unwrap()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.screen_size_sampler_descriptor_set.clone(),
            )
            .unwrap()
            .begin_render_pass(
                RenderPassBeginInfo {
                    render_pass: self.render_pass.clone(),
                    clear_values: vec![None],
                    ..RenderPassBeginInfo::framebuffer(framebuffer)
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .unwrap();

        let mut loaded_texture = None;

        for shape in context.tessellate(full_output.shapes, full_output.pixels_per_point) {
            match shape.primitive {
                Primitive::Mesh(mesh) => {
                    let indexes_size = size_of::<u32>() * mesh.indices.len();
                    let vertexes_size = size_of::<Vertex>() * mesh.vertices.len();

                    let layout = DeviceLayout::new(
                        NonZero::<DeviceSize>::new((indexes_size + vertexes_size) as DeviceSize)
                            .unwrap(),
                        VERTEX_INDEX_DEVICE_ALIGNMENT.max(VERTEX_DEVICE_ALIGNMENT),
                    )
                    .unwrap();

                    // Upload data
                    let buffer = self.vertex_buffer_pool.allocate(layout).unwrap();

                    let (vertex_buffer_view, index_buffer_view) =
                        buffer.split_at(vertexes_size as u64);

                    let vertex_buffer_view = vertex_buffer_view.reinterpret::<[Vertex]>();
                    let index_buffer_view = index_buffer_view.reinterpret::<[u32]>();

                    let mut vertex_buffer_view_guard = vertex_buffer_view.write().unwrap();
                    let mut index_buffer_view_guard = index_buffer_view.write().unwrap();

                    for (source, destination) in mesh
                        .vertices
                        .into_iter()
                        .map(Vertex::from)
                        .zip(vertex_buffer_view_guard.iter_mut())
                    {
                        *destination = source;
                    }
                    index_buffer_view_guard.copy_from_slice(&mesh.indices);

                    drop(vertex_buffer_view_guard);
                    drop(index_buffer_view_guard);

                    if loaded_texture != Some(mesh.texture_id) {
                        let (_, descriptor_set) = self
                            .textures
                            .get(&mesh.texture_id)
                            .expect("Mesh reference missing texture");

                        command_buffer
                            .bind_descriptor_sets(
                                PipelineBindPoint::Graphics,
                                self.pipeline.layout().clone(),
                                1,
                                descriptor_set.clone(),
                            )
                            .unwrap();

                        loaded_texture = Some(mesh.texture_id);
                    }

                    command_buffer
                        .bind_vertex_buffers(0, vertex_buffer_view)
                        .unwrap()
                        .bind_index_buffer(index_buffer_view.clone())
                        .unwrap();

                    // This is safe as long as egui gives valid indexes
                    // The actual process to verify this would consume too much cpu time to be done
                    unsafe {
                        command_buffer
                            .draw_indexed(
                                u32::try_from(index_buffer_view.len())
                                    .expect("Far too many indexes"),
                                1,
                                0,
                                0,
                                0,
                            )
                            .unwrap()
                    };
                }
                Primitive::Callback(..) => {
                    tracing::warn!("Epaint callbacks are ignored");
                }
            }
        }

        command_buffer
            .end_render_pass(SubpassEndInfo::default())
            .unwrap();
    }
}
