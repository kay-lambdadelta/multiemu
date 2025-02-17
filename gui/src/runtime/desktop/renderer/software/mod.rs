use crate::gui::software_rasterizer::SoftwareEguiRenderer;
use crate::rendering_backend::RenderingBackendState;
use multiemu_config::Environment;
use multiemu_machine::component::ComponentId;
use multiemu_machine::display::software::SoftwareRendering;
use multiemu_machine::display::RenderBackend;
use nalgebra::{DMatrixViewMut, Vector2};
use palette::Srgba;
use softbuffer::{Context, Surface};
use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZero;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use winit::window::Window;

pub struct SoftwareRenderingRuntime {
    surface: Surface<Arc<Window>, Arc<Window>>,
    display_api_handle: Arc<Window>,
    egui_renderer: SoftwareEguiRenderer,
    component_framebuffers: HashMap<
        ComponentId,
        Rc<RefCell<<SoftwareRendering as RenderBackend>::ComponentFramebuffer>>,
    >,
    environment: Arc<RwLock<Environment>>,
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
        let window_dimensions = Vector2::new(
            NonZero::new(window_dimensions.width).unwrap(),
            NonZero::new(window_dimensions.height).unwrap(),
        );

        let context = Context::new(display_api_handle.clone())?;
        let mut surface = Surface::new(&context, display_api_handle.clone())?;

        surface.resize(window_dimensions.x, window_dimensions.y)?;

        Ok(Self {
            surface,
            display_api_handle,
            egui_renderer: SoftwareEguiRenderer::default(),
            component_framebuffers: HashMap::new(),
            environment,
        })
    }

    fn component_initialization_data(
        &self,
    ) -> Rc<<Self::RenderBackend as RenderBackend>::ComponentInitializationData> {
        Rc::default()
    }

    fn redraw(&mut self) {
        let window_dimensions = self.display_api_handle.inner_size();
        let window_dimensions =
            Vector2::new(window_dimensions.width, window_dimensions.height).cast::<usize>();

        // Skip rendering if impossible window size
        if window_dimensions.min() == 0 {
            return;
        }

        let mut surface_buffer = self.surface.buffer_mut().unwrap();
        let mut surface_buffer_view = DMatrixViewMut::from_slice(
            bytemuck::cast_slice_mut(surface_buffer.as_mut()),
            window_dimensions.x,
            window_dimensions.y,
        );
        surface_buffer_view.fill(Srgba::<u8>::new(0, 0, 0, 0xff));

        for (index, component_framebuffer) in self.component_framebuffers.values().enumerate() {
            let component_framebuffer = component_framebuffer.borrow();

            let component_display_buffer_size =
                Vector2::new(component_framebuffer.nrows(), component_framebuffer.ncols())
                    .cast::<u16>();

            let scaling = window_dimensions
                .cast::<f32>()
                .component_div(&component_display_buffer_size.cast::<f32>());

            // Iterate over each pixel in the display component buffer
            for x in 0..component_framebuffer.nrows() {
                for y in 0..component_framebuffer.ncols() {
                    let source_pixel = component_framebuffer[(x, y)];

                    let dest_start = Vector2::new(x, y)
                        .cast::<f32>()
                        .component_mul(&scaling)
                        .map(f32::round)
                        .try_cast::<usize>()
                        .unwrap()
                        .zip_map(&window_dimensions, |dest_dim, window_dim| {
                            dest_dim.min(window_dim)
                        });

                    let dest_end = Vector2::new(x, y)
                        .cast::<f32>()
                        .add_scalar(1.0)
                        .component_mul(&scaling)
                        .map(f32::round)
                        .try_cast::<usize>()
                        .unwrap()
                        .zip_map(&window_dimensions, |dest_dim, window_dim| {
                            dest_dim.min(window_dim)
                        });

                    // Fill the destination pixels with the source pixel
                    let mut destination_pixels = surface_buffer_view.view_mut(
                        (dest_start.x, dest_start.y),
                        (dest_end.x - dest_start.x, dest_end.y - dest_start.y),
                    );

                    destination_pixels.fill(source_pixel);
                }
            }
        }

        surface_buffer.present().unwrap();
    }

    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: egui::FullOutput) {
        let window_dimensions = self.display_api_handle.inner_size();
        let window_dimensions = Vector2::new(window_dimensions.width, window_dimensions.height);

        let mut surface_buffer = self.surface.buffer_mut().unwrap();
        let surface_buffer_view = DMatrixViewMut::from_slice(
            bytemuck::cast_slice_mut(surface_buffer.as_mut()),
            window_dimensions.x as usize,
            window_dimensions.y as usize,
        );

        self.egui_renderer
            .render(egui_context, surface_buffer_view, full_output);

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
    }
}
