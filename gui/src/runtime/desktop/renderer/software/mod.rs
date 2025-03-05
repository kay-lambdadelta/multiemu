use crate::gui::software_rendering::SoftwareEguiRenderer;
use crate::rendering_backend::RenderingBackendState;
use multiemu_config::Environment;
use multiemu_machine::Machine;
use multiemu_machine::component::ComponentId;
use multiemu_machine::display::RenderBackend;
use multiemu_machine::display::software::SoftwareRendering;
use nalgebra::{DMatrix, DMatrixViewMut, Vector2};
use palette::Srgba;
use palette::cast::Packed;
use palette::rgb::channels::Argb;
use softbuffer::{Context, Surface};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use winit::window::Window;

pub struct SoftwareRenderingRuntime {
    surface: Surface<Arc<Window>, Arc<Window>>,
    display_api_handle: Arc<Window>,
    egui_renderer: SoftwareEguiRenderer,
    previously_recorded_size: Vector2<u16>,
    environment: Arc<RwLock<Environment>>,
    previously_seen_frames: HashMap<ComponentId, DMatrix<Srgba<u8>>>,
}

impl RenderingBackendState for SoftwareRenderingRuntime {
    type RenderBackend = SoftwareRendering;
    type DisplayApiHandle = Arc<Window>;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        _preferred_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        _required_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window_dimensions = display_api_handle.inner_size();
        let window_dimensions = Vector2::new(window_dimensions.width, window_dimensions.height);

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
            previously_seen_frames: HashMap::new(),
        })
    }

    fn component_initialization_data(
        &self,
    ) -> Arc<<Self::RenderBackend as RenderBackend>::ComponentInitializationData> {
        Arc::default()
    }

    fn redraw(&mut self, machine: &Machine<Self::RenderBackend>) {
        if self.previously_recorded_size.min() == 0 {
            return;
        }

        let mut surface_buffer = self.surface.buffer_mut().unwrap();
        let mut surface_buffer_view = DMatrixViewMut::from_slice(
            bytemuck::cast_slice_mut(surface_buffer.as_mut()),
            self.previously_recorded_size.x as usize,
            self.previously_recorded_size.y as usize,
        );
        surface_buffer_view.fill(Packed::<Argb, u32>::pack(Srgba::new(0, 0, 0, 0xff)));

        for (component_id, framebuffer_receiver) in &machine.component_framebuffers {
            if let Ok(framebuffer) = framebuffer_receiver.try_recv() {
                self.previously_seen_frames
                    .insert(*component_id, framebuffer);
            }
        }

        let integer_scaling = self
            .environment
            .read()
            .unwrap()
            .graphics_setting
            .integer_scaling;

        if integer_scaling {
            for (index, component_framebuffer) in self.previously_seen_frames.iter() {
                let component_display_buffer_size =
                    Vector2::new(component_framebuffer.nrows(), component_framebuffer.ncols())
                        .cast::<u16>();

                let scaling = self
                    .previously_recorded_size
                    .component_div(&component_display_buffer_size)
                    .cast::<usize>();

                // Iterate over each pixel in the display component buffer
                for x in 0..component_framebuffer.nrows() {
                    for y in 0..component_framebuffer.ncols() {
                        let source_pixel = component_framebuffer[(x, y)];

                        let dest_start = Vector2::new(x, y).component_mul(&scaling).zip_map(
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
            }
        } else {
            for (index, component_framebuffer) in self.previously_seen_frames.iter() {
                let component_display_buffer_size =
                    Vector2::new(component_framebuffer.nrows(), component_framebuffer.ncols())
                        .cast::<u16>();

                let scaling = self
                    .previously_recorded_size
                    .cast::<f32>()
                    .component_div(&component_display_buffer_size.cast::<f32>());

                // Iterate over each pixel in the display component buffer
                for x in 0..component_framebuffer.nrows() {
                    for y in 0..component_framebuffer.ncols() {
                        let source_pixel = component_framebuffer[(x, y)];

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
        }

        self.display_api_handle.pre_present_notify();
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

        self.display_api_handle.pre_present_notify();
        surface_buffer.present().unwrap();
    }

    fn surface_resized(&mut self) {
        let window_dimensions = self.display_api_handle.inner_size();
        let window_dimensions = Vector2::new(window_dimensions.width, window_dimensions.height);

        self.surface
            .resize(
                window_dimensions.x.try_into().unwrap(),
                window_dimensions.y.try_into().unwrap(),
            )
            .unwrap();
        self.previously_recorded_size = window_dimensions.cast();
    }
}
