use super::{PpuDisplayBackend, SupportedGraphicsApiPpu};
use crate::ppu::{VISIBLE_SCANLINE_LENGTH, region::Region};
use multiemu_graphics::{
    GraphicsApi,
    software::{InitializationData, Software},
};
use nalgebra::DMatrix;
use palette::{Srgba, named::BLACK};
use std::fmt::Debug;

pub struct SoftwareState {
    pub staging_buffer: DMatrix<Srgba<u8>>,
    pub framebuffer: DMatrix<Srgba<u8>>,
}

// elide the buffers

impl Debug for SoftwareState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoftwareState").finish()
    }
}

impl<R: Region> PpuDisplayBackend<R> for SoftwareState {
    type GraphicsApi = Software;

    fn new(_: InitializationData) -> Self {
        let staging_buffer = DMatrix::from_element(
            VISIBLE_SCANLINE_LENGTH as usize,
            R::VISIBLE_SCANLINES as usize,
            BLACK.into(),
        );

        SoftwareState {
            framebuffer: staging_buffer.clone(),
            staging_buffer,
        }
    }

    fn modify_staging_buffer(
        &mut self,
        callback: impl FnOnce(nalgebra::DMatrixViewMut<'_, Srgba<u8>>),
    ) {
        callback(self.staging_buffer.as_view_mut());
    }

    fn commit_staging_buffer(&mut self) {
        self.framebuffer.copy_from(&self.staging_buffer);
    }

    fn access_framebuffer(
        &mut self,
        callback: impl FnOnce(&<Software as GraphicsApi>::FramebufferTexture),
    ) {
        callback(&self.framebuffer);
    }
}

impl SupportedGraphicsApiPpu for Software {
    type Backend<R: Region> = SoftwareState;
}
