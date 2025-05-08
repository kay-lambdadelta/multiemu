use super::Chip8Kind;
use bitvec::{order::Msb0, view::BitView};
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    display::backend::software::SoftwareRendering,
};
use nalgebra::{DMatrix, DMatrixViewMut, Point2, Vector2};
use num::rational::Ratio;
use palette::{Srgb, Srgba};
use serde::{Deserialize, Serialize};
use std::{
    cell::{OnceCell, RefCell},
    io::{Read, Write},
    ops::Deref,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use versions::SemVer;

mod software;
#[cfg(all(feature = "vulkan", platform_desktop))]
mod vulkan;

#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    screen_buffer: DMatrix<Srgb<u8>>,
}

pub struct Chip8Display {
    state: OnceCell<Box<dyn Chip8DisplayBackend>>,
    modified: RefCell<bool>,
    mode: Arc<Mutex<Chip8Kind>>,
    pub vsync_occurred: Arc<AtomicBool>,
}

impl Chip8Display {
    pub fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        tracing::trace!(
            "Drawing sprite at position {} of dimensions 8x{}",
            position,
            sprite.len()
        );

        let mode = self.mode.lock().unwrap();

        let position = match mode.deref() {
            Chip8Kind::Chip8 | Chip8Kind::Chip48 => Point2::new(position.x % 64, position.y % 32),
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

    fn save(&self, mut entry: &mut dyn Write) -> Result<SemVer, Box<dyn std::error::Error>> {
        let snapshot = Snapshot {
            screen_buffer: self.state.get().unwrap().save_screen_contents(),
        };

        bincode::serde::encode_into_std_write(snapshot, &mut entry, bincode::config::standard())?;

        Ok(SemVer::new("1.0.0").unwrap())
    }

    fn load(
        &self,
        mut entry: &mut dyn Read,
        version: SemVer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, SemVer::new("1.0.0").unwrap());

        let snapshot: Snapshot =
            bincode::serde::decode_from_std_read(&mut entry, bincode::config::standard())?;

        self.state
            .get()
            .unwrap()
            .load_screen_contents(snapshot.screen_buffer);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct Chip8DisplayQuirks {
    pub force_mode: Option<Chip8Kind>,
}

impl FromConfig for Chip8Display {
    type Config = ();
    type Quirks = Chip8DisplayQuirks;

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        _config: Self::Config,
        quirks: Self::Quirks,
    ) {
        let mode = Arc::new(Mutex::new(quirks.force_mode.unwrap_or(Chip8Kind::Chip8)));
        let vsync = Arc::new(AtomicBool::new(false));

        let component_builder = component_builder
            .insert_task(Ratio::from_integer(60), {
                let vsync = vsync.clone();

                move |display: &Chip8Display, _period| {
                    // Only update it once and if the thing is actually updated
                    if *display.modified.borrow().deref() {
                        display.state.get().unwrap().commit_display();
                        *display.modified.borrow_mut() = false;
                    }

                    vsync.store(true, Ordering::Relaxed);
                }
            })
            .set_display_config::<SoftwareRendering>(None, None, software::set_display_data);

        #[cfg(all(feature = "vulkan", platform_desktop))]
        let component_builder = {
            use multiemu_machine::display::backend::vulkan::VulkanRendering;
            component_builder.set_display_config::<VulkanRendering>(
                None,
                None,
                vulkan::set_display_data,
            )
        };

        component_builder.build(Chip8Display {
            state: OnceCell::default(),
            modified: RefCell::new(true),
            mode,
            vsync_occurred: vsync,
        });
    }
}

trait Chip8DisplayBackend {
    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool;
    fn clear_display(&self);
    fn save_screen_contents(&self) -> DMatrix<Srgb<u8>>;
    fn load_screen_contents(&self, buffer: DMatrix<Srgb<u8>>);
    fn commit_display(&self);
}

fn draw_sprite_common(
    position: Point2<u8>,
    sprite: &[u8],
    mut framebuffer: DMatrixViewMut<'_, Srgba<u8>>,
) -> bool {
    let position = position.cast();
    let dimensions = Vector2::new(8, sprite.len());

    if dimensions.min() == 0 {
        return false;
    }

    let mut collided = false;
    for (y, sprite_row) in sprite.view_bits::<Msb0>().chunks(8).enumerate() {
        for (x, sprite_pixel) in sprite_row.iter().enumerate() {
            let position = position + Vector2::new(x, y);

            if position.x >= 64 || position.y >= 32 {
                continue;
            }

            let old_sprite_pixel =
                framebuffer[(position.x, position.y)] != Srgba::new(0, 0, 0, 255);

            if *sprite_pixel && old_sprite_pixel {
                collided = true;
            }

            framebuffer[(position.x, position.y)] = if *sprite_pixel ^ old_sprite_pixel {
                Srgba::new(255, 255, 255, 255)
            } else {
                Srgba::new(0, 0, 0, 255)
            };
        }
    }

    collided
}
