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
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

mod software;
#[cfg(feature = "vulkan")]
mod vulkan;

const LORES: Vector2<u8> = Vector2::new(64, 32);
const HIRES: Vector2<u8> = Vector2::new(128, 64);

#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    screen_buffer: DMatrix<Srgb<u8>>,
}

#[derive(Debug)]
pub struct Chip8Display<R: SupportedGraphicsApiChip8Display> {
    backend: Mutex<R::Backend>,
    /// The cpu reads this to see if it can continue execution post draw call
    pub vsync_occurred: AtomicBool,
    hires: AtomicBool,
    config: Chip8DisplayConfig,
}

impl<R: SupportedGraphicsApiChip8Display> Chip8Display<R> {
    pub fn set_hires(&self, is_hires: bool) {
        let mut backend_guard = self.backend.lock().unwrap();

        if self.config.clear_on_resolution_change {
            self.clear_display();
        }

        backend_guard.resize(if is_hires { HIRES } else { LORES }.cast());
        self.hires.store(is_hires, Ordering::Relaxed);
    }

    pub fn draw_supersized_sprite(&self, position: Point2<u8>, sprite: [u8; 32]) -> bool {
        tracing::debug!(
            "Drawing sprite at position {} of dimensions 16x16",
            position,
        );
        let mut backend_guard = self.backend.lock().unwrap();

        let screen_size = if self.hires.load(Ordering::Relaxed) {
            HIRES
        } else {
            LORES
        };
        let position = Point2::new(position.x % screen_size.x, position.y % screen_size.y).cast();
        self.vsync_occurred.store(false, Ordering::Relaxed);

        let mut hit_detection = false;

        backend_guard.modify_staging_buffer(|mut framebuffer| {
            for (y, sprite_row) in sprite.view_bits::<Msb0>().chunks(16).enumerate() {
                for (x, sprite_pixel) in sprite_row.iter().enumerate() {
                    let position = position + Vector2::new(x, y);

                    if position.x >= screen_size.x as usize || position.y >= screen_size.y as usize
                    {
                        continue;
                    }

                    let old_sprite_pixel =
                        framebuffer[(position.x, position.y)] != Srgba::new(0, 0, 0, 255);

                    if *sprite_pixel && old_sprite_pixel {
                        hit_detection = true;
                    }

                    framebuffer[(position.x, position.y)] = if *sprite_pixel ^ old_sprite_pixel {
                        Srgba::new(255, 255, 255, 255)
                    } else {
                        Srgba::new(0, 0, 0, 255)
                    };
                }
            }
        });

        hit_detection
    }

    pub fn draw_sprite(&self, position: Point2<u8>, sprite: &[u8]) -> bool {
        tracing::debug!(
            "Drawing sprite at position {} of dimensions 8x{}",
            position,
            sprite.len()
        );
        let mut backend_guard = self.backend.lock().unwrap();

        let screen_size = if self.hires.load(Ordering::Relaxed) {
            HIRES
        } else {
            LORES
        };
        self.vsync_occurred.store(false, Ordering::Relaxed);

        let position = Point2::new(position.x % screen_size.x, position.y % screen_size.y).cast();
        let dimensions = Vector2::new(8, sprite.len());

        if dimensions.min() == 0 {
            return false;
        }
        let mut hit_detection = false;

        backend_guard.modify_staging_buffer(|mut framebuffer| {
            for (y, sprite_row) in sprite.view_bits::<Msb0>().chunks(8).enumerate() {
                for (x, sprite_pixel) in sprite_row.iter().enumerate() {
                    let position = position + Vector2::new(x, y);

                    if position.x >= screen_size.x as usize || position.y >= screen_size.y as usize
                    {
                        continue;
                    }

                    let old_sprite_pixel =
                        framebuffer[(position.x, position.y)] != Srgba::new(0, 0, 0, 255);

                    if *sprite_pixel && old_sprite_pixel {
                        hit_detection = true;
                    }

                    framebuffer[(position.x, position.y)] = if *sprite_pixel ^ old_sprite_pixel {
                        Srgba::new(255, 255, 255, 255)
                    } else {
                        Srgba::new(0, 0, 0, 255)
                    };
                }
            }
        });

        hit_detection
    }

    pub fn clear_display(&self) {
        tracing::trace!("Clearing display");
        let mut backend_guard = self.backend.lock().unwrap();

        backend_guard.modify_staging_buffer(|mut framebuffer| {
            framebuffer.fill(Srgba::new(0, 0, 0, 255));
        });
    }
}

impl<R: SupportedGraphicsApiChip8Display> Component for Chip8Display<R> {
    fn on_reset(&self) {
        self.clear_display();
    }
}

pub(crate) trait Chip8DisplayBackend: Debug + 'static {
    type GraphicsApi: GraphicsApi;

    fn new(initialization_data: <Self::GraphicsApi as GraphicsApi>::InitializationData) -> Self;
    fn resize(&mut self, resolution: Vector2<usize>);
    fn modify_staging_buffer(&mut self, callback: impl FnOnce(DMatrixViewMut<'_, Srgba<u8>>));
    fn commit_staging_buffer(&mut self);
    fn get_framebuffer(
        &mut self,
        callback: impl FnOnce(&<Self::GraphicsApi as GraphicsApi>::FramebufferTexture),
    );
}

#[derive(Debug, Default)]
pub struct Chip8DisplayConfig {
    pub clear_on_resolution_change: bool,
}

impl<P: Platform<GraphicsApi: SupportedGraphicsApiChip8Display>> ComponentConfig<P>
    for Chip8DisplayConfig
{
    type Component = Chip8Display<P::GraphicsApi>;

    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) {
        let graphics_initialization_data = component_builder
            .essentials()
            .component_graphics_initialization_data
            .clone();

        let (component_builder, _) = component_builder
            .insert_task(Ratio::from_integer(60), {
                let component = component_ref.clone();

                // We ignore the time slice and only commit the buffer once
                move |_: NonZero<u32>| {
                    component
                        .interact(|display| {
                            display.vsync_occurred.store(true, Ordering::Relaxed);
                            display.backend.lock().unwrap().commit_staging_buffer();
                        })
                        .unwrap();
                }
            })
            .insert_display(Chip8DisplayCallback {
                component: component_ref.clone(),
            });

        component_builder.build(Chip8Display {
            backend: Mutex::new(Chip8DisplayBackend::new(graphics_initialization_data)),
            hires: AtomicBool::new(false),
            vsync_occurred: AtomicBool::new(false),
            config: self,
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
                display.backend.lock().unwrap().get_framebuffer(callback);
            })
            .unwrap();
    }
}
