use super::Chip8Kind;
use bitvec::{order::Msb0, view::BitView};
use multiemu_graphics::GraphicsApi;
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, ComponentRef},
    graphics::DisplayCallback,
    platform::Platform,
};
use nalgebra::{DMatrix, DMatrixViewMut, Point2, Vector2};
use num::rational::Ratio;
use palette::{Srgb, Srgba};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    num::NonZero,
    ops::Deref,
    sync::{
        Arc, Mutex, OnceLock,
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
pub struct Chip8Display<R: SupportedGraphicsApiChip8Display> {
    /// Actually just initialized ones
    backend: OnceLock<R::Backend>,
    mode: Arc<Mutex<Chip8Kind>>,
    graphics_initialization_data: R::InitializationData,
    /// The cpu reads this to see if it can continue execution post draw call
    pub vsync_occurred: Arc<AtomicBool>,
}

impl<R: SupportedGraphicsApiChip8Display> Chip8Display<R> {
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

        let mut hit_detection = false;

        self.backend
            .get()
            .unwrap()
            .modify_staging_buffer(|framebuffer| {
                hit_detection = draw_sprite(position, sprite, framebuffer);
            });

        hit_detection
    }

    pub fn clear_display(&self) {
        tracing::trace!("Clearing display");

        self.backend
            .get()
            .unwrap()
            .modify_staging_buffer(|mut framebuffer| {
                framebuffer.fill(Srgba::new(0, 0, 0, 255));
            });
    }
}

impl<R: SupportedGraphicsApiChip8Display> Component for Chip8Display<R> {
    fn on_reset(&self) {
        self.clear_display();
    }

    fn on_runtime_ready(&self) {
        let backend = Chip8DisplayBackend::new(self.graphics_initialization_data.clone());
        self.backend.set(backend).unwrap();
    }
}

pub(crate) trait Chip8DisplayBackend: Debug + 'static {
    type GraphicsApi: GraphicsApi;

    fn new(initialization_data: <Self::GraphicsApi as GraphicsApi>::InitializationData) -> Self;
    fn modify_staging_buffer(&self, callback: impl FnOnce(DMatrixViewMut<'_, Srgba<u8>>));
    fn commit_staging_buffer(&self);
    fn get_framebuffer(
        &self,
        callback: impl FnOnce(&<Self::GraphicsApi as GraphicsApi>::FramebufferTexture),
    );
}

#[inline]
fn draw_sprite(
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

impl<P: Platform<GraphicsApi: SupportedGraphicsApiChip8Display>> ComponentConfig<P>
    for Chip8DisplayConfig
{
    type Component = Chip8Display<P::GraphicsApi>;

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) {
        let vsync_occurred = Arc::new(AtomicBool::default());
        let backend = OnceLock::default();

        let graphics_initialization_data = component_builder
            .essentials()
            .component_graphics_initialization_data
            .clone();

        let (component_builder, _) = component_builder
            .insert_task(Ratio::from_integer(60), {
                let vsync = vsync_occurred.clone();
                let component = component_ref.clone();

                // We ignore the time slice and only commit the buffer once
                move |_: NonZero<u32>| {
                    component
                        .interact(|display| {
                            display.backend.get().unwrap().commit_staging_buffer();
                        })
                        .unwrap();

                    vsync.store(true, Ordering::Relaxed);
                }
            })
            .insert_display(Chip8DisplayCallback {
                component: component_ref.clone(),
            });

        component_builder.build(Chip8Display {
            backend,
            mode: Arc::default(),
            vsync_occurred,
            graphics_initialization_data,
        })
    }
}

pub(crate) trait SupportedGraphicsApiChip8Display: GraphicsApi {
    type Backend: Chip8DisplayBackend<GraphicsApi = Self>;
}

#[derive(Debug)]
pub struct Chip8DisplayCallback<R: SupportedGraphicsApiChip8Display> {
    pub component: ComponentRef<Chip8Display<R>>,
}

impl<R: SupportedGraphicsApiChip8Display> DisplayCallback<R> for Chip8DisplayCallback<R> {
    fn access_framebuffer(
        &self,
        callback: Box<dyn FnOnce(&<R as GraphicsApi>::FramebufferTexture) + '_>,
    ) {
        self.component
            .interact_local(|display| {
                display.backend.get().unwrap().get_framebuffer(callback);
            })
            .unwrap();
    }
}
