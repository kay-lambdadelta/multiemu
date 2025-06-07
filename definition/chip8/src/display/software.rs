use super::{Chip8DisplayBackend, SupportedGraphicsApiChip8Display};
use multiemu_graphics::Software;
use multiemu_runtime::component::RuntimeEssentials;
use nalgebra::{DMatrix, DMatrixViewMut};
use palette::Srgba;

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: DMatrix<Srgba<u8>>,
    pub framebuffer: DMatrix<Srgba<u8>>,
}

impl Chip8DisplayBackend<Software> for SoftwareState {
    fn new(_essentials: &RuntimeEssentials<Software>) -> Self {
        let staging_buffer = DMatrix::from_element(64, 32, Srgba::new(0, 0, 0, 0xff));

        Self {
            framebuffer: staging_buffer.clone(),
            staging_buffer,
        }
    }

    fn modify_staging_buffer(&mut self, callback: impl FnOnce(DMatrixViewMut<Srgba<u8>>)) {
        callback(self.staging_buffer.as_view_mut());
    }

    fn commit_staging_buffer(&mut self) {
        self.framebuffer.copy_from(&self.staging_buffer);
    }

    fn get_framebuffer(&mut self) -> &DMatrix<Srgba<u8>> {
        &self.framebuffer
    }
}

impl SupportedGraphicsApiChip8Display for Software {
    type Backend = SoftwareState;
}
