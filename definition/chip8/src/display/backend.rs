use crate::display::CHIP8_DIMENSIONS;
use multiemu_runtime::graphics::ComponentGraphicsInitializationData;
use nalgebra::DMatrix;
use palette::Srgba;
use std::sync::Mutex;
use wgpu::{
    Extent3d, TexelCopyBufferLayout, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

#[derive(Debug)]
pub(crate) struct Chip8DisplayBackend {
    pub staging_buffer: Mutex<DMatrix<Srgba<u8>>>,
    pub framebuffer: Texture,
    wgpu_data: ComponentGraphicsInitializationData,
}

impl Chip8DisplayBackend {
    pub fn new(wgpu_data: ComponentGraphicsInitializationData) -> Self {
        let ComponentGraphicsInitializationData { device, .. } = wgpu_data.clone();

        let staging_buffer = Mutex::new(DMatrix::from_element(
            CHIP8_DIMENSIONS.x as usize,
            CHIP8_DIMENSIONS.y as usize,
            Srgba::new(0, 0, 0, u8::MAX),
        ));

        let framebuffer = device.create_texture(&TextureDescriptor {
            label: Some("chip8_framebuffer"),
            size: Extent3d {
                width: CHIP8_DIMENSIONS.x as u32,
                height: CHIP8_DIMENSIONS.y as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Self {
            staging_buffer,
            framebuffer,
            wgpu_data,
        }
    }

    pub fn commit_staging_buffer(&self) {
        self.wgpu_data.queue.write_texture(
            self.framebuffer.as_image_copy(),
            bytemuck::cast_slice(self.staging_buffer.lock().unwrap().as_slice()),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size_of::<Srgba<u8>>() as u32 * CHIP8_DIMENSIONS.x as u32),
                rows_per_image: None,
            },
            self.framebuffer.size(),
        );

        self.wgpu_data.queue.submit([]);
    }
}
