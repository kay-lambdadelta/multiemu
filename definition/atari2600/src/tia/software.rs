use super::{
    SCANLINE_LENGTH, SupportedRenderApiTia, TiaDisplayBackend, color::TiaColor, region::Region,
};
use multiemu_machine::{
    component::RuntimeEssentials,
    display::backend::{ComponentFramebuffer, software::SoftwareRendering},
};
use nalgebra::{DMatrix, Point2};
use palette::{Srgb, Srgba};
use std::{cell::RefCell, sync::Arc};

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: RefCell<DMatrix<Srgba<u8>>>,
    pub framebuffer: ComponentFramebuffer<SoftwareRendering>,
}

impl<R: Region> TiaDisplayBackend<R, SoftwareRendering> for SoftwareState {
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

    fn draw(&self, position: Point2<u16>, color: TiaColor) {
        let real_color = R::color_to_srgb(color);

        let mut staging_buffer = self.staging_buffer.borrow_mut();

        let color = Srgba::new(real_color.red, real_color.green, real_color.blue, 0xff);
        staging_buffer[(position.x as usize, position.y as usize)] = color;
    }

    fn save_screen_contents(&self) -> DMatrix<Srgb<u8>> {
        let staging_buffer = self.staging_buffer.borrow();

        staging_buffer.map(|pixel| Srgb::new(pixel.red, pixel.green, pixel.blue))
    }

    fn load_screen_contents(&self, buffer: DMatrix<Srgb<u8>>) {
        let mut staging_buffer = self.staging_buffer.borrow_mut();

        *staging_buffer = buffer.map(|pixel| Srgba::new(pixel.red, pixel.green, pixel.blue, 0xff));
    }

    fn commit_display(&self) {
        let staging_buffer = self.staging_buffer.borrow();
        self.framebuffer.store(Arc::new(staging_buffer.clone()));
    }
}

impl SupportedRenderApiTia for SoftwareRendering {
    type Backend<R: Region> = SoftwareState;
}
