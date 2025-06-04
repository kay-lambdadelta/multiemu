use super::{Chip8DisplayBackend, SupportedRenderApiChip8Display, draw_sprite_common};
use crate::Chip8Kind;
use multiemu_runtime::{
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

impl Chip8DisplayBackend<SoftwareRendering> for SoftwareState {
    fn new(
        _essentials: &RuntimeEssentials<SoftwareRendering>,
    ) -> (Self, ComponentFramebuffer<SoftwareRendering>) {
        let staging_buffer = DMatrix::from_element(64, 32, Srgba::new(0, 0, 0, 0xff));
        let framebuffer = ComponentFramebuffer::new(Arc::new(staging_buffer.clone()));

        (
            Self {
                staging_buffer: RefCell::new(staging_buffer),
                framebuffer: framebuffer.clone(),
            },
            framebuffer,
        )
    }

    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        let mut staging_buffer = self.staging_buffer.borrow_mut();

        draw_sprite_common(position, sprite, staging_buffer.as_view_mut())
    }

    fn clear_display(&self) {
        let mut staging_buffer = self.staging_buffer.borrow_mut();

        staging_buffer.fill(Srgba::new(0, 0, 0, 255));
    }

    fn set_mode(&mut self, mode: Chip8Kind) {}

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

impl SupportedRenderApiChip8Display for SoftwareRendering {
    type Backend = SoftwareState;
}
