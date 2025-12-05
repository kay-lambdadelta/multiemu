use std::{
    any::Any,
    fmt::Debug,
    io::{Read, Write},
};

use bitvec::{order::Msb0, view::BitView};
use multiemu_runtime::{
    component::{
        Component, ComponentConfig, ComponentVersion, LateInitializedData, ResourcePath,
        SynchronizationContext,
    },
    graphics::GraphicsApi,
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    platform::Platform,
    scheduler::Period,
};
use nalgebra::{DMatrix, DMatrixView, DMatrixViewMut, Point2, Vector2};
use palette::{
    Srgba,
    named::{BLACK, WHITE},
};
use serde::{Deserialize, Serialize};

mod software;
#[cfg(feature = "vulkan")]
mod vulkan;

const LORES: Vector2<u8> = Vector2::new(64, 32);
const HIRES: Vector2<u8> = Vector2::new(128, 64);

#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    screen_buffer: DMatrix<Srgba<u8>>,
    vsync_occurred: bool,
    hires: bool,
}

#[derive(Debug)]
pub struct Chip8Display<R: SupportedGraphicsApiChip8Display> {
    backend: Option<R::Backend>,
    /// The cpu reads this to see if it can continue execution post draw call
    pub vsync_occurred: bool,
    hires: bool,
    config: Chip8DisplayConfig,
}

impl<R: SupportedGraphicsApiChip8Display> Chip8Display<R> {
    pub fn set_hires(&mut self, is_hires: bool) {
        if self.config.clear_on_resolution_change {
            self.clear_display();
        }

        self.backend
            .as_mut()
            .unwrap()
            .resize(if is_hires { HIRES } else { LORES }.cast());
        self.hires = is_hires;
    }

    pub fn draw_supersized_sprite(&mut self, position: Point2<u8>, sprite: [u8; 32]) -> bool {
        tracing::debug!(
            "Drawing sprite at position {} of dimensions 16x16",
            position,
        );

        let screen_size = if self.hires { HIRES } else { LORES };
        let position = Point2::new(position.x % screen_size.x, position.y % screen_size.y).cast();
        self.vsync_occurred = false;

        let mut hit_detection = false;

        self.backend
            .as_mut()
            .unwrap()
            .interact_staging_buffer_mut(|mut framebuffer| {
                for (y, sprite_row) in sprite.view_bits::<Msb0>().chunks(16).enumerate() {
                    for (x, sprite_pixel) in sprite_row.iter().enumerate() {
                        let position = position + Vector2::new(x, y);

                        if position.x >= screen_size.x as usize
                            || position.y >= screen_size.y as usize
                        {
                            continue;
                        }

                        let old_sprite_pixel =
                            framebuffer[(position.x, position.y)] != BLACK.into();

                        if *sprite_pixel && old_sprite_pixel {
                            hit_detection = true;
                        }

                        framebuffer[(position.x, position.y)] = if *sprite_pixel ^ old_sprite_pixel
                        {
                            WHITE
                        } else {
                            BLACK
                        }
                        .into();
                    }
                }
            });

        hit_detection
    }

    pub fn draw_sprite(&mut self, position: Point2<u8>, sprite: &[u8]) -> bool {
        tracing::debug!(
            "Drawing sprite at position {} of dimensions 8x{}",
            position,
            sprite.len()
        );

        let screen_size = if self.hires { HIRES } else { LORES };
        self.vsync_occurred = false;

        let position = Point2::new(position.x % screen_size.x, position.y % screen_size.y).cast();
        let dimensions = Vector2::new(8, sprite.len());

        if dimensions.min() == 0 {
            return false;
        }
        let mut hit_detection = false;

        self.backend
            .as_mut()
            .unwrap()
            .interact_staging_buffer_mut(|mut framebuffer| {
                for (y, sprite_row) in sprite.view_bits::<Msb0>().chunks(8).enumerate() {
                    for (x, sprite_pixel) in sprite_row.iter().enumerate() {
                        let position = position + Vector2::new(x, y);

                        if position.x >= screen_size.x as usize
                            || position.y >= screen_size.y as usize
                        {
                            continue;
                        }

                        let old_sprite_pixel =
                            framebuffer[(position.x, position.y)] != BLACK.into();

                        if *sprite_pixel && old_sprite_pixel {
                            hit_detection = true;
                        }

                        framebuffer[(position.x, position.y)] = if *sprite_pixel ^ old_sprite_pixel
                        {
                            WHITE
                        } else {
                            BLACK
                        }
                        .into();
                    }
                }
            });

        hit_detection
    }

    pub fn clear_display(&mut self) {
        tracing::trace!("Clearing display");

        self.backend
            .as_mut()
            .unwrap()
            .interact_staging_buffer_mut(|mut framebuffer| {
                framebuffer.fill(BLACK.into());
            });
    }
}

impl<R: SupportedGraphicsApiChip8Display> Component for Chip8Display<R> {
    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);
        let snapshot: Snapshot =
            bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;

        self.set_hires(snapshot.hires);

        self.backend
            .as_mut()
            .unwrap()
            .interact_staging_buffer_mut(|mut framebuffer| {
                framebuffer.copy_from(&snapshot.screen_buffer);
            });

        self.vsync_occurred = snapshot.vsync_occurred;

        Ok(())
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let screen_size = if self.hires { HIRES } else { LORES }.cast();

        let mut screen_buffer = DMatrix::from_element(screen_size.x, screen_size.y, BLACK.into());

        self.backend
            .as_ref()
            .unwrap()
            .interact_staging_buffer(|framebuffer| {
                screen_buffer.copy_from(&framebuffer);
            });

        let snapshot = Snapshot {
            screen_buffer,
            hires: self.hires,
            vsync_occurred: self.vsync_occurred,
        };
        bincode::serde::encode_into_std_write(&snapshot, &mut writer, bincode::config::standard())?;

        Ok(())
    }

    fn access_framebuffer(&mut self, _path: &ResourcePath) -> &dyn Any {
        self.backend.as_mut().unwrap().access_framebuffer()
    }

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        if context.allocate_period(Period::ONE / 60) {
            self.vsync_occurred = true;
            self.backend.as_mut().unwrap().commit_staging_buffer();
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= Period::ONE / 60
    }
}

pub(crate) trait Chip8DisplayBackend: Send + Sync + Debug + 'static {
    type GraphicsApi: GraphicsApi;

    fn new(initialization_data: <Self::GraphicsApi as GraphicsApi>::InitializationData) -> Self;
    fn resize(&mut self, resolution: Vector2<usize>);
    fn interact_staging_buffer(&self, callback: impl FnOnce(DMatrixView<'_, Srgba<u8>>));
    fn interact_staging_buffer_mut(&mut self, callback: impl FnOnce(DMatrixViewMut<'_, Srgba<u8>>));
    fn commit_staging_buffer(&mut self);
    fn access_framebuffer(&mut self) -> &<Self::GraphicsApi as GraphicsApi>::FramebufferTexture;
}

#[derive(Debug, Default)]
pub struct Chip8DisplayConfig {
    pub clear_on_resolution_change: bool,
}

impl<P: Platform<GraphicsApi: SupportedGraphicsApiChip8Display>> ComponentConfig<P>
    for Chip8DisplayConfig
{
    type Component = Chip8Display<P::GraphicsApi>;

    fn late_initialize(component: &mut Self::Component, data: &LateInitializedData<P>) {
        component.backend = Some(Chip8DisplayBackend::new(
            data.component_graphics_initialization_data.clone(),
        ));
    }

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        component_builder
            .set_scheduler_participation(SchedulerParticipation::OnDemand)
            .insert_display("display");

        Ok(Chip8Display {
            backend: None,
            hires: false,
            vsync_occurred: false,
            config: self,
        })
    }
}

pub(crate) trait SupportedGraphicsApiChip8Display: GraphicsApi {
    type Backend: Chip8DisplayBackend<GraphicsApi = Self>;
}
