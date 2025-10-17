use crate::display::LORES;

use super::{Chip8DisplayBackend, SupportedGraphicsApiChip8Display};
use multiemu_runtime::graphics::{
    GraphicsApi,
    software::{InitializationData, Software},
};
use nalgebra::{DMatrix, DMatrixView, DMatrixViewMut, Vector2};
use palette::{Srgba, named::BLACK};

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: DMatrix<Srgba<u8>>,
    pub framebuffer: DMatrix<Srgba<u8>>,
}

impl Chip8DisplayBackend for SoftwareState {
    type GraphicsApi = Software;

    fn new(_initialization_data: InitializationData) -> Self {
        let staging_buffer =
            DMatrix::from_element(LORES.x as usize, LORES.y as usize, BLACK.into());

        Self {
            staging_buffer: staging_buffer.clone(),
            framebuffer: staging_buffer,
        }
    }

    fn resize(&mut self, resolution: Vector2<usize>) {
        self.staging_buffer
            .resize_mut(resolution.x, resolution.y, BLACK.into());

        self.framebuffer = self.staging_buffer.clone();
    }

    fn interact_staging_buffer(&self, callback: impl FnOnce(DMatrixView<Srgba<u8>>)) {
        callback(self.staging_buffer.as_view());
    }

    fn interact_staging_buffer_mut(&mut self, callback: impl FnOnce(DMatrixViewMut<Srgba<u8>>)) {
        callback(self.staging_buffer.as_view_mut());
    }

    fn commit_staging_buffer(&mut self) {
        self.framebuffer.copy_from(&self.staging_buffer);
    }

    fn access_framebuffer(
        &mut self,
        callback: impl FnOnce(&<Self::GraphicsApi as GraphicsApi>::FramebufferTexture),
    ) {
        callback(&self.framebuffer)
    }
}

impl SupportedGraphicsApiChip8Display for Software {
    type Backend = SoftwareState;
}
