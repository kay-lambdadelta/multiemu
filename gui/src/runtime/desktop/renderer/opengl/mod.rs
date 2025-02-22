use crate::rendering_backend::RenderingBackendState;
use egui::FullOutput;
use glium::Display;
use glium::backend::{Context, Facade};
use glutin::prelude::{GlConfig, GlDisplay, NotCurrentGlContext};
use glutin::surface::WindowSurface;
use gui::OpenglEguiRenderer;
use multiemu_config::Environment;
use multiemu_machine::Machine;
use multiemu_machine::display::RenderBackend;
use multiemu_machine::display::opengl::{OpenGlComponentInitializationData, OpenGlRendering};
use nalgebra::Vector2;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
#[allow(deprecated)]
use winit::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

mod gui;

pub struct OpenGlRenderingRuntime {
    pub display_api_handle: Arc<Window>,
    pub display: Display<WindowSurface>,
    pub context: Rc<Context>,
    pub gui_renderer: OpenglEguiRenderer,
}

impl RenderingBackendState for OpenGlRenderingRuntime {
    type RenderBackend = OpenGlRendering;
    type DisplayApiHandle = Arc<Window>;

    fn new(
        display_api_handle: Self::DisplayApiHandle,
        _preferred_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        _required_extensions: <Self::RenderBackend as RenderBackend>::ContextExtensionSpecification,
        _environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn Error>> {
        let window_size = display_api_handle.inner_size();
        let window_size = Vector2::new(window_size.width, window_size.height);

        #[cfg(target_os = "linux")]
        let display = unsafe {
            glutin::display::Display::new(
                #[allow(deprecated)]
                display_api_handle.raw_display_handle()?,
                glutin::display::DisplayApiPreference::GlxThenEgl(Box::new(
                    winit::platform::x11::register_xlib_error_hook,
                )),
            )
        }?;

        let potential_configs: Vec<_> =
            unsafe { display.find_configs(glutin::config::ConfigTemplateBuilder::new().build()) }?
                .filter(|config| {
                    config.srgb_capable()
                        && config
                            .config_surface_types()
                            .contains(glutin::config::ConfigSurfaceTypes::WINDOW)
                })
                .collect();

        let best_config = potential_configs.iter().max_by_key(|config| {
            let mut overall_score = 0;

            if config.hardware_accelerated() {
                overall_score += 10;
            }

            overall_score
        });

        let config = best_config
            .or_else(|| potential_configs.first())
            .expect("No OpenGL config found");

        #[allow(deprecated)]
        let context_attributes = glutin::context::ContextAttributesBuilder::new()
            .build(Some(display_api_handle.raw_window_handle()?));

        let not_current_context = unsafe { display.create_context(config, &context_attributes)? };

        #[allow(deprecated)]
        let surface_attributes =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(
                    display_api_handle.raw_window_handle()?,
                    window_size.x.try_into().unwrap(),
                    window_size.y.try_into().unwrap(),
                );

        let surface = unsafe {
            display
                .create_window_surface(config, &surface_attributes)
                .unwrap()
        };
        let context = not_current_context.make_current(&surface).unwrap();
        let display = Display::from_context_surface(context, surface)?;

        let version = display.get_opengl_version_string();
        tracing::info!("Found opengl {} implementation", version);

        let context = display.get_context().clone();

        Ok(Self {
            display_api_handle,
            gui_renderer: OpenglEguiRenderer::new(context.clone()),
            context,
            display,
        })
    }

    fn component_initialization_data(
        &self,
    ) -> Arc<<Self::RenderBackend as RenderBackend>::ComponentInitializationData> {
        Arc::new(OpenGlComponentInitializationData {
            context: self.context.clone(),
        })
    }

    fn redraw(&mut self, machine: &Machine<Self::RenderBackend>) {
        todo!()
    }

    fn redraw_menu(&mut self, egui_context: &egui::Context, full_output: FullOutput) {
        let mut render_buffer = self.display.draw();

        self.gui_renderer
            .render(egui_context, &mut render_buffer, full_output);

        render_buffer.finish().unwrap();
    }

    fn surface_resized(&mut self) {
        let window_dimensions = self.display_api_handle.inner_size();
        let window_dimensions = Vector2::new(window_dimensions.width, window_dimensions.height);

        self.display
            .resize((window_dimensions.x, window_dimensions.y));
    }
}
