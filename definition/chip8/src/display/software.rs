use super::{Chip8Display, Chip8DisplayBackend, draw_sprite_common};
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

impl Chip8DisplayBackend for SoftwareState {
    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        let mut staging_buffer = self.staging_buffer.borrow_mut();

        draw_sprite_common(position, sprite, staging_buffer.as_view_mut())
    }

    fn clear_display(&self) {
        let mut staging_buffer = self.staging_buffer.borrow_mut();

        staging_buffer.fill(Srgba::new(0, 0, 0, 255));
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

pub fn set_display_data(
    display: &Chip8Display,
    _initialization_data: Rc<<SoftwareRendering as RenderBackend>::ComponentInitializationData>,
) -> Rc<SoftwareComponentFramebuffer> {
    let staging_buffer = DMatrix::from_element(64, 32, Srgba::new(0, 0, 0, 0xff));
    let framebuffer = Rc::new(RefCell::new(staging_buffer.clone()));

    let _ = display.state.set(Box::new(SoftwareState {
        staging_buffer: RefCell::new(staging_buffer),
        framebuffer: framebuffer.clone(),
    }));

    framebuffer
}
