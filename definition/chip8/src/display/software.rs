use super::{Chip8DisplayBackend, SupportedGraphicsApiChip8Display};
use multiemu_graphics::{
    GraphicsApi,
    software::{InitializationData, Software},
};
use nalgebra::{DMatrix, DMatrixViewMut};
use palette::Srgba;
use std::{ops::DerefMut, sync::Mutex};

#[derive(Debug)]
struct Inner {
    pub staging_buffer: DMatrix<Srgba<u8>>,
    pub framebuffer: DMatrix<Srgba<u8>>,
}

#[derive(Debug)]
pub struct SoftwareState {
    inner: Mutex<Inner>,
}

impl Chip8DisplayBackend for SoftwareState {
    type GraphicsApi = Software;

    fn new(_initialization_data: InitializationData) -> Self {
        let staging_buffer = DMatrix::from_element(64, 32, Srgba::new(0, 0, 0, 0xff));

        Self {
            inner: Mutex::new(Inner {
                staging_buffer: staging_buffer.clone(),
                framebuffer: staging_buffer,
            }),
        }
    }

    fn modify_staging_buffer(&self, callback: impl FnOnce(DMatrixViewMut<Srgba<u8>>)) {
        let mut inner_guard = self.inner.lock().unwrap();

        callback(inner_guard.staging_buffer.as_view_mut());
    }

    fn commit_staging_buffer(&self) {
        let mut inner_guard = self.inner.lock().unwrap();
        let inner_guard = inner_guard.deref_mut();

        inner_guard
            .framebuffer
            .copy_from(&inner_guard.staging_buffer);
    }

    fn get_framebuffer(
        &self,
        callback: impl FnOnce(&<Self::GraphicsApi as GraphicsApi>::FramebufferTexture),
    ) {
        let inner_guard = self.inner.lock().unwrap();

        callback(&inner_guard.framebuffer)
    }
}

impl SupportedGraphicsApiChip8Display for Software {
    type Backend = SoftwareState;
}
