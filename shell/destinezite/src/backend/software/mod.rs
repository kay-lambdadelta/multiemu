use crate::windowing::{DesktopPlatform, WinitWindow};
use multiemu_frontend::{
    DisplayApiHandle, GraphicsRuntime, gui_software_rendering::SoftwareEguiRenderer,
};
use multiemu_runtime::{
    environment::Environment,
    graphics::{GraphicsApi, software::Software},
    machine::Machine,
};
use nalgebra::{DMatrixViewMut, Vector2};
use palette::{cast::Packed, named::BLACK, rgb::channels::Argb};
use softbuffer::{Context, Surface};
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

pub struct SoftwareGraphicsRuntime {
    surface: Surface<WinitWindow, WinitWindow>,
    display_api_handle: WinitWindow,
    egui_renderer: SoftwareEguiRenderer,
    previously_recorded_size: Vector2<u16>,
    environment: Arc<RwLock<Environment>>,
}

impl Debug for SoftwareGraphicsRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoftwareRenderingRuntime")
            .field("display_api_handle", &self.display_api_handle)
            .field("egui_renderer", &self.egui_renderer)
            .field("previously_recorded_size", &self.previously_recorded_size)
            .field("environment", &self.environment)
            .finish()
    }
}

impl GraphicsRuntime<DesktopPlatform<Software, Self>> for SoftwareGraphicsRuntime {
    type DisplayApiHandle = WinitWindow;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        _required_features: <Software as GraphicsApi>::Features,
        _preferred_features: <Software as GraphicsApi>::Features,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window_dimensions = display_api_handle.dimensions();

        let context = Context::new(display_api_handle.clone())?;
        let mut surface = Surface::new(&context, display_api_handle.clone())?;

        surface.resize(
            window_dimensions.x.try_into().unwrap(),
            window_dimensions.y.try_into().unwrap(),
        )?;

        Ok(Self {
            surface,
            display_api_handle,
            egui_renderer: SoftwareEguiRenderer::default(),
            environment,
            previously_recorded_size: window_dimensions.cast(),
        })
    }

    fn component_initialization_data(&self) -> <Software as GraphicsApi>::InitializationData {
        Default::default()
    }

    fn redraw(&mut self, machine: &Machine<DesktopPlatform<Software, Self>>) {
        if self.previously_recorded_size.min() == 0 {
            return;
        }

        let mut surface_buffer = self.surface.buffer_mut().unwrap();
        let mut surface_buffer_view = DMatrixViewMut::from_slice(
            bytemuck::cast_slice_mut(surface_buffer.as_mut()),
            self.previously_recorded_size.x as usize,
            self.previously_recorded_size.y as usize,
        );
        surface_buffer_view.fill(Packed::<Argb, u32>::pack(BLACK.into()));

        let integer_scaling = self
            .environment
            .read()
            .unwrap()
            .graphics_setting
            .integer_scaling;
        for display_path in machine.displays.iter() {
            machine
                .component_registry
                .interact_dyn_mut(&display_path.component, |component| {
                    component.access_framebuffer(
                        display_path,
                        Box::new(|display| {
                            let display: &<Software as GraphicsApi>::FramebufferTexture =
                                display.downcast_ref().unwrap();

                            if integer_scaling {
                                let component_display_buffer_size =
                                    Vector2::new(display.nrows(), display.ncols()).cast::<u16>();

                                let scaling = self
                                    .previously_recorded_size
                                    .component_div(&component_display_buffer_size)
                                    .cast::<usize>();

                                // Iterate over each pixel in the display component buffer
                                for x in 0..display.nrows() {
                                    for y in 0..display.ncols() {
                                        let source_pixel = display[(x, y)];

                                        let dest_start =
                                            Vector2::new(x, y).component_mul(&scaling).zip_map(
                                                &self.previously_recorded_size.cast::<usize>(),
                                                |dest_dim, window_dim| dest_dim.min(window_dim),
                                            );

                                        let dest_end = Vector2::new(x, y)
                                            .add_scalar(1)
                                            .component_mul(&scaling)
                                            .zip_map(
                                                &self.previously_recorded_size.cast::<usize>(),
                                                |dest_dim, window_dim| dest_dim.min(window_dim),
                                            );

                                        let mut destination_pixels = surface_buffer_view.view_mut(
                                            (dest_start.x, dest_start.y),
                                            (dest_end.x - dest_start.x, dest_end.y - dest_start.y),
                                        );

                                        destination_pixels.fill(Packed::pack(source_pixel));
                                    }
                                }
                            } else {
                                let component_display_buffer_size =
                                    Vector2::new(display.nrows(), display.ncols()).cast::<u16>();

                                let scaling = self
                                    .previously_recorded_size
                                    .cast::<f32>()
                                    .component_div(&component_display_buffer_size.cast::<f32>());

                                // Iterate over each pixel in the display component buffer
                                for x in 0..display.nrows() {
                                    for y in 0..display.ncols() {
                                        let source_pixel = display[(x, y)];

                                        let dest_start = Vector2::new(x, y)
                                            .cast::<f32>()
                                            .component_mul(&scaling)
                                            .try_cast::<usize>()
                                            .unwrap()
                                            .zip_map(
                                                &self.previously_recorded_size.cast::<usize>(),
                                                |dest_dim, window_dim| dest_dim.min(window_dim),
                                            );

                                        let dest_end = Vector2::new(x, y)
                                            .cast::<f32>()
                                            .add_scalar(1.0)
                                            .component_mul(&scaling)
                                            .try_cast::<usize>()
                                            .unwrap()
                                            .zip_map(
                                                &self.previously_recorded_size.cast::<usize>(),
                                                |dest_dim, window_dim| dest_dim.min(window_dim),
                                            );

                                        // Fill the destination pixels with the source pixel
                                        let mut destination_pixels = surface_buffer_view.view_mut(
                                            (dest_start.x, dest_start.y),
                                            (dest_end.x - dest_start.x, dest_end.y - dest_start.y),
                                        );

                                        destination_pixels.fill(Packed::pack(source_pixel));
                                    }
                                }
                            }
                        }),
                    );
                })
                .unwrap();
        }

        self.display_api_handle.inner().pre_present_notify();
        surface_buffer.present().unwrap();
    }

    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: egui::FullOutput) {
        if self.previously_recorded_size.min() == 0 {
            return;
        }

        let mut surface_buffer = self.surface.buffer_mut().unwrap();
        let surface_buffer_view = DMatrixViewMut::from_slice(
            bytemuck::cast_slice_mut(surface_buffer.as_mut()),
            self.previously_recorded_size.x as usize,
            self.previously_recorded_size.y as usize,
        );

        self.egui_renderer
            .render::<Argb>(egui_context, surface_buffer_view, full_output);

        self.display_api_handle.inner().pre_present_notify();
        surface_buffer.present().unwrap();
    }

    fn display_resized(&mut self) {
        let window_dimensions = self.display_api_handle.dimensions();

        self.surface
            .resize(
                window_dimensions.x.try_into().unwrap(),
                window_dimensions.y.try_into().unwrap(),
            )
            .unwrap();
        self.previously_recorded_size = window_dimensions.cast();
    }
}
