use super::{SupportedGraphicsApiTia, TiaDisplayBackend};
use crate::tia::{region::Region, VISIBLE_SCANLINE_LENGTH};
use multiemu_graphics::{
    GraphicsApi,
    software::{InitializationData, Software},
};
use nalgebra::DMatrix;
use palette::Srgba;

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: DMatrix<Srgba<u8>>,
    pub framebuffer: DMatrix<Srgba<u8>>,
}

impl<R: Region> TiaDisplayBackend<R> for SoftwareState {
    type GraphicsApi = Software;

    fn new(_: InitializationData) -> Self {
        let staging_buffer = DMatrix::from_element(
            VISIBLE_SCANLINE_LENGTH as usize,
            R::TOTAL_SCANLINES as usize,
            Srgba::new(0, 0, 0, 0xff),
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

impl SupportedGraphicsApiTia for Software {
    type Backend<R: Region> = SoftwareState;
}
