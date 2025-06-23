use super::{SupportedGraphicsApiTia, TiaDisplayBackend};
use crate::tia::{SCANLINE_LENGTH, region::Region};
use multiemu_graphics::{
    GraphicsApi,
    software::{InitializationData, Software},
};
use nalgebra::DMatrix;
use palette::Srgba;
use std::cell::RefCell;

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: RefCell<DMatrix<Srgba<u8>>>,
    pub framebuffer: RefCell<DMatrix<Srgba<u8>>>,
}

impl<R: Region> TiaDisplayBackend<R> for SoftwareState {
    type GraphicsApi = Software;

    fn new(_: InitializationData) -> Self {
        let staging_buffer = DMatrix::from_element(
            SCANLINE_LENGTH as usize,
            R::TOTAL_SCANLINES as usize,
            Srgba::new(0, 0, 0, 0xff),
        );

        SoftwareState {
            framebuffer: RefCell::new(staging_buffer.clone()),
            staging_buffer: RefCell::new(staging_buffer),
        }
    }

    fn modify_staging_buffer(
        &self,
        callback: impl FnOnce(nalgebra::DMatrixViewMut<'_, Srgba<u8>>),
    ) {
        let mut staging_buffer_guard = self.staging_buffer.borrow_mut();
        callback(staging_buffer_guard.as_view_mut());
    }

    fn commit_staging_buffer(&self) {
        let staging_buffer_guard = self.staging_buffer.borrow();
        let mut framebuffer_guard = self.framebuffer.borrow_mut();

        framebuffer_guard.copy_from(&staging_buffer_guard);
    }

    fn access_framebuffer(
        &self,
        callback: impl FnOnce(&<Software as GraphicsApi>::FramebufferTexture),
    ) {
        let framebuffer_guard = self.framebuffer.borrow();
        callback(&framebuffer_guard);
    }
}

impl SupportedGraphicsApiTia for Software {
    type Backend<R: Region> = SoftwareState;
}
