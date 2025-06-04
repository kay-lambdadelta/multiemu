use super::{
    FramebufferGuard, SCANLINE_LENGTH, SupportedRenderApiTia, TiaDisplayBackend, region::Region,
};
use multiemu_runtime::{
    component::RuntimeEssentials,
    display::backend::{ComponentFramebuffer, software::SoftwareRendering},
};
use nalgebra::{DMatrix, DMatrixViewMut};
use palette::Srgba;
use std::{
    cell::{RefCell, RefMut},
    sync::Arc,
};

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: RefCell<DMatrix<Srgba<u8>>>,
    pub framebuffer: ComponentFramebuffer<SoftwareRendering>,
}

impl FramebufferGuard for RefMut<'_, DMatrix<Srgba<u8>>> {
    fn get(&mut self) -> DMatrixViewMut<'_, Srgba<u8>> {
        self.as_view_mut()
    }
}

impl<R: Region> TiaDisplayBackend<R, SoftwareRendering> for SoftwareState {
    type FramebufferGuard<'a> = RefMut<'a, DMatrix<Srgba<u8>>>;

    fn new(
        _essentials: &RuntimeEssentials<SoftwareRendering>,
    ) -> (Self, ComponentFramebuffer<SoftwareRendering>) {
        let staging_buffer = DMatrix::from_element(
            SCANLINE_LENGTH as usize,
            R::TOTAL_SCANLINES as usize,
            Srgba::new(0, 0, 0, 0xff),
        );
        let framebuffer = ComponentFramebuffer::new(Arc::new(staging_buffer.clone()));

        (
            SoftwareState {
                staging_buffer: RefCell::new(staging_buffer),
                framebuffer: framebuffer.clone(),
            },
            framebuffer,
        )
    }

    fn lock_framebuffer(&self) -> Self::FramebufferGuard<'_> {
        self.staging_buffer.borrow_mut()
    }

    fn commit_display(&self) {
        let staging_buffer = self.staging_buffer.borrow();
        self.framebuffer.store(Arc::new(staging_buffer.clone()));
    }
}

impl SupportedRenderApiTia for SoftwareRendering {
    type Backend<R: Region> = SoftwareState;
}
