use super::Chip8Kind;
use bitvec::{order::Msb0, view::BitView};
use downcast_rs::Downcast;
use multiemu_machine::builder::ComponentBuilder;
use multiemu_machine::component::{Component, FromConfig, RuntimeEssentials};
use multiemu_machine::display::software::SoftwareRendering;
use nalgebra::{DMatrix, DMatrixViewMut, Point2, Vector2};
use num::rational::Ratio;
use palette::Srgba;
use serde::{Deserialize, Serialize};
use std::cell::{OnceCell, RefCell};
use std::ops::Deref;
use std::sync::Arc;

#[cfg(all(feature = "opengl", platform_desktop))]
mod opengl;
mod software;
#[cfg(all(feature = "vulkan", platform_desktop))]
mod vulkan;

#[derive(Debug, Serialize, Deserialize)]
pub struct Chip8DisplaySnapshot {
    screen_buffer: DMatrix<Srgba<u8>>,
}

pub struct Chip8Display {
    config: Chip8DisplayConfig,
    state: OnceCell<Box<dyn Chip8DisplayBackend>>,
    modified: RefCell<bool>,
}

impl Chip8Display {
    pub fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        tracing::trace!(
            "Drawing sprite at position {} of dimensions 8x{}",
            position,
            sprite.len()
        );

        let position = match self.config.kind {
            Chip8Kind::Chip8 | Chip8Kind::Chip48 => Point2::new(position.x % 63, position.y % 31),
            Chip8Kind::SuperChip8 => todo!(),
            _ => todo!(),
        };

        *self.modified.borrow_mut() = true;
        self.state.get().unwrap().draw_sprite(position, sprite)
    }

    pub fn clear_display(&self) {
        tracing::trace!("Clearing display");

        *self.modified.borrow_mut() = true;
        self.state.get().unwrap().clear_display();
    }
}

impl Component for Chip8Display {
    fn reset(&self) {
        self.clear_display();
    }
}

#[derive(Debug)]
pub struct Chip8DisplayConfig {
    pub kind: Chip8Kind,
}

impl FromConfig for Chip8Display {
    type Config = Chip8DisplayConfig;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
    ) {
        let component_builder = component_builder
            .insert_task(
                Ratio::from_integer(60),
                |display: &Chip8Display, _period| {
                    // Only update it once and if the thing is actually updated
                    if *display.modified.borrow().deref() {
                        display.state.get().unwrap().commit_display();
                        *display.modified.borrow_mut() = false;
                    }
                },
            )
            .set_display_config::<SoftwareRendering>(None, None, software::set_display_data);

        #[cfg(all(feature = "vulkan", platform_desktop))]
        let component_builder = {
            use multiemu_machine::display::vulkan::VulkanRendering;
            component_builder.set_display_config::<VulkanRendering>(
                None,
                None,
                vulkan::set_display_data,
            )
        };

        #[cfg(all(feature = "opengl", platform_desktop))]
        let component_builder = {
            use multiemu_machine::display::opengl::OpenGlRendering;
            component_builder.set_display_config::<OpenGlRendering>(
                None,
                None,
                opengl::set_display_data,
            )
        };

        component_builder.build(Chip8Display {
            config,
            state: OnceCell::default(),
            modified: RefCell::new(true),
        });
    }
}

trait Chip8DisplayBackend: Downcast {
    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool;
    fn clear_display(&self);
    fn save_screen_contents(&self) -> DMatrix<Srgba<u8>>;
    fn load_screen_contents(&self, buffer: DMatrix<Srgba<u8>>);
    fn commit_display(&self);
}

fn draw_sprite_common(
    position: Point2<u8>,
    sprite: &[u8],
    mut framebuffer: DMatrixViewMut<'_, Srgba<u8>>,
) -> bool {
    let mut collided = false;
    let position = position.cast();

    for (y, sprite_row) in sprite.view_bits::<Msb0>().chunks(8).enumerate() {
        for (x, sprite_pixel) in sprite_row.iter().enumerate() {
            let coord = position + Vector2::new(x, y);

            if coord.x >= 64 || coord.y >= 32 {
                continue;
            }

            let old_sprite_pixel =
                framebuffer[(coord.x, coord.y)] == Srgba::new(255, 255, 255, 255);

            if *sprite_pixel && old_sprite_pixel {
                collided = true;
            }

            framebuffer[(coord.x, coord.y)] = if *sprite_pixel ^ old_sprite_pixel {
                Srgba::new(255, 255, 255, 255)
            } else {
                Srgba::new(0, 0, 0, 255)
            };
        }
    }

    collided
}
