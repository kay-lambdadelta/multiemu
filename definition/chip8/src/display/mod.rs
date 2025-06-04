use super::Chip8Kind;
use bitvec::{order::Msb0, view::BitView};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, RuntimeEssentials},
    display::backend::{ComponentFramebuffer, RenderApi},
};
use nalgebra::{DMatrix, DMatrixViewMut, Point2, Vector2};
use num::rational::Ratio;
use palette::{Srgb, Srgba};
use serde::{Deserialize, Serialize};
use std::{
    cell::{OnceCell, RefCell},
    fmt::Debug,
    ops::Deref,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

mod software;
#[cfg(feature = "vulkan")]
mod vulkan;

const CHIP8_DIMENSIONS: Vector2<u8> = Vector2::new(64, 32);
const SUPER_CHIP8_DIMENSIONS: Vector2<u8> = Vector2::new(128, 64);

#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    screen_buffer: DMatrix<Srgb<u8>>,
}

#[derive(Debug)]
pub struct Chip8Display<R: SupportedRenderApiChip8Display> {
    backend: OnceCell<R::Backend>,
    modified: RefCell<bool>,
    mode: Arc<Mutex<Chip8Kind>>,
    pub vsync_occurred: Arc<AtomicBool>,
}

impl<R: SupportedRenderApiChip8Display> Chip8Display<R> {
    pub fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        tracing::trace!(
            "Drawing sprite at position {} of dimensions 8x{}",
            position,
            sprite.len()
        );
        self.vsync_occurred.store(false, Ordering::Relaxed);
        let mode = self.mode.lock().unwrap();

        let position = match mode.deref() {
            Chip8Kind::Chip8 | Chip8Kind::Chip48 => Point2::new(
                position.x % CHIP8_DIMENSIONS.x,
                position.y % CHIP8_DIMENSIONS.y,
            ),
            Chip8Kind::SuperChip8 => Point2::new(
                position.x % SUPER_CHIP8_DIMENSIONS.x,
                position.y % SUPER_CHIP8_DIMENSIONS.y,
            ),
            _ => todo!(),
        };

        *self.modified.borrow_mut() = true;
        self.backend.get().unwrap().draw_sprite(position, sprite)
    }

    pub fn clear_display(&self) {
        tracing::trace!("Clearing display");

        *self.modified.borrow_mut() = true;
        self.backend.get().unwrap().clear_display();
    }
}

impl<R: SupportedRenderApiChip8Display> Component for Chip8Display<R> {
    fn reset(&self) {
        self.clear_display();
    }
}

pub(crate) trait Chip8DisplayBackend<R: RenderApi>: Sized + Debug + 'static {
    fn new(essentials: &RuntimeEssentials<R>) -> (Self, ComponentFramebuffer<R>);
    fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool;
    fn set_mode(&mut self, mode: Chip8Kind);
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

#[derive(Debug, Default)]
pub struct Chip8DisplayConfig;

impl<
    R: SupportedRenderApiChip8Display,
    B: ComponentBuilder<Component = Chip8Display<R>, RenderApi = R>,
> ComponentConfig<B> for Chip8DisplayConfig
{
    type Component = Chip8Display<R>;

    fn build_component(self, component_builder: B) -> B::BuildOutput {
        let vsync_occurred: Arc<AtomicBool> = Arc::default();

        let essentials = component_builder.essentials();

        let component_builder = component_builder.insert_display_config(
            None,
            None,
            move |component: &Self::Component| {
                let (backend, framebuffer) =
                    <R::Backend as Chip8DisplayBackend<R>>::new(essentials.as_ref());

                component.backend.set(backend).unwrap();

                framebuffer
            },
        );

        component_builder
            .insert_task(Ratio::from_integer(60), {
                let vsync = vsync_occurred.clone();

                move |display: &Chip8Display<R>, _period| {
                    // Only update it once and if the thing is actually updated
                    if *display.modified.borrow() {
                        display.backend.get().unwrap().commit_display();
                        *display.modified.borrow_mut() = false;
                    }

                    vsync.store(true, Ordering::Relaxed);
                }
            })
            .build(Chip8Display {
                backend: OnceCell::default(),
                modified: RefCell::new(true),
                mode: Arc::default(),
                vsync_occurred,
            })
    }
}

pub(crate) trait SupportedRenderApiChip8Display: RenderApi {
    type Backend: Chip8DisplayBackend<Self>;
}
