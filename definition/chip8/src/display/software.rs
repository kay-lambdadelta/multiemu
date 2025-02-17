use super::{draw_sprite_common, Chip8DisplayBackend};
use nalgebra::{DMatrix, Point2};
use palette::Srgba;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct SoftwareState {
    pub staging_buffer: RefCell<DMatrix<Srgba<u8>>>,
    pub render_image: Rc<RefCell<DMatrix<Srgba<u8>>>>,
}

impl Chip8DisplayBackend for SoftwareState {
    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        let mut staging_buffer = self.staging_buffer.borrow_mut();

        draw_sprite_common(position, sprite, staging_buffer.as_view_mut())
    }

    fn clear_display(&self) {
        self.render_image
            .borrow_mut()
            .fill(Srgba::new(0, 0, 0, 255));
    }

    fn save_screen_contents(&self) -> DMatrix<Srgba<u8>> {
        self.render_image.borrow().clone()
    }

    fn load_screen_contents(&self, buffer: DMatrix<Srgba<u8>>) {
        self.render_image.borrow_mut().clone_from(&buffer);
    }

    fn commit_display(&self) {
        self.render_image
            .borrow_mut()
            .copy_from(&self.staging_buffer.borrow());
    }
}
