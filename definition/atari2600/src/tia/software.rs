use super::{SCANLINE_LENGTH, State, Tia, TiaDisplayBackend, region::Region};
use multiemu_machine::display::backend::{
    RenderBackend,
    software::{SoftwareComponentFramebuffer, SoftwareRendering},
};
use nalgebra::{DMatrix, Point2};
use palette::{Srgb, Srgba};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: RefCell<DMatrix<Srgba<u8>>>,
    pub framebuffer: Rc<RefCell<DMatrix<Srgba<u8>>>>,
}

impl<R: Region> TiaDisplayBackend<R> for SoftwareState {
    fn draw(&self, state: &State, position: Point2<u16>, hue: u8, luminosity: u8) {
        let real_color = R::color_to_srgb(hue, luminosity);

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
        let staging_buffer = self.staging_buffer.borrow_mut();
        let mut framebuffer = self.framebuffer.borrow_mut();

        framebuffer.copy_from(&staging_buffer);
    }
}

pub fn set_display_data<R: Region>(
    display: &Tia<R>,
    _initialization_data: Rc<<SoftwareRendering as RenderBackend>::ComponentInitializationData>,
) -> Rc<SoftwareComponentFramebuffer> {
    let staging_buffer = DMatrix::from_element(
        SCANLINE_LENGTH as usize,
        R::TOTAL_SCANLINES as usize,
        Srgba::new(0, 0, 0, 0xff),
    );
    let framebuffer = Rc::new(RefCell::new(staging_buffer.clone()));

    let _ = display.display_backend.set(Box::new(SoftwareState {
        staging_buffer: RefCell::new(staging_buffer),
        framebuffer: framebuffer.clone(),
    }));

    framebuffer
}
