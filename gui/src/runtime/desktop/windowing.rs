use super::{
    RuntimeBoundMessage,
    input::keyboard::{KEYBOARD_ID, winit2key},
};
use crate::{
    rendering_backend::{DisplayApiHandle, RenderingBackendState},
    runtime::{
        Platform,
        desktop::{audio::CpalAudioRuntime, input::gamepad::gamepad_task},
        state::MainRuntime,
    },
    write_environment,
};
use egui::ViewportId;
use multiemu_config::ENVIRONMENT_LOCATION;
use multiemu_graphics::GraphicsApi;
use multiemu_input::InputState;
use nalgebra::Vector2;
use std::{fmt::Debug, fs::File, ops::Deref, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

impl DisplayApiHandle for Arc<Window> {
    fn dimensions(&self) -> nalgebra::Vector2<u32> {
        let size = self.inner_size();
        Vector2::new(size.width, size.height)
    }
}

pub struct DesktopPlatform<RS: RenderingBackendState> {
    runtime: MainRuntime<RS, CpalAudioRuntime>,
    egui_winit: Option<egui_winit::State>,
}

impl<RS: RenderingBackendState> Debug for DesktopPlatform<RS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopPlatform")
            .field("runtime", &self.runtime)
            .finish()
    }
}

impl<R: GraphicsApi, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, GraphicsApi = R>>
    Platform<RS, CpalAudioRuntime> for DesktopPlatform<RS>
{
    fn run(runtime: MainRuntime<RS, CpalAudioRuntime>) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::with_user_event().build()?;
        {
            let event_loop_proxy = event_loop.create_proxy();

            std::thread::Builder::new()
                .name("gamepad".to_string())
                .spawn(move || {
                    tracing::debug!("Starting up gamepad thread");

                    gamepad_task(event_loop_proxy);

                    tracing::debug!("Shutting down gamepad thread");
                })?;
        }

        let mut me = Self {
            egui_winit: None,
            runtime,
        };
        event_loop.run_app(&mut me)?;

        Ok(())
    }
}

impl<R: GraphicsApi, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, GraphicsApi = R>>
    ApplicationHandler<RuntimeBoundMessage> for DesktopPlatform<RS>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let display_api_handle = setup_window(event_loop);

        tracing::debug!("Scale factor: {}", display_api_handle.scale_factor());
        let egui_winit = egui_winit::State::new(
            self.runtime.egui_context.clone(),
            ViewportId::ROOT,
            &display_api_handle,
            Some(display_api_handle.scale_factor() as f32),
            None,
            None,
        );
        self.runtime.set_display_api_handle(display_api_handle);
        self.egui_winit = Some(egui_winit);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let display_api_handle = self
            .runtime
            .display_api_handle()
            .expect("Display API handle not initialized");

        let egui_winit = self
            .egui_winit
            .as_mut()
            .expect("egui_winit not initialized");

        if self.runtime.was_egui_context_reset() {
            *egui_winit = egui_winit::State::new(
                self.runtime.egui_context.clone(),
                ViewportId::ROOT,
                &display_api_handle,
                Some(display_api_handle.scale_factor() as f32),
                None,
                None,
            );
        }

        let _ = egui_winit.on_window_event(&display_api_handle, &event);

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Window close requested");

                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic,
            } => {
                if is_synthetic {
                    return;
                }

                if let Some(input) = winit2key(event.physical_key) {
                    let state = InputState::Digital(event.state.is_pressed());

                    self.runtime.insert_input(KEYBOARD_ID, input, state);
                }
            }
            WindowEvent::RedrawRequested => {
                self.runtime
                    .redraw(Some(egui_winit.take_egui_input(&display_api_handle)));
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeBoundMessage) {
        match event {
            RuntimeBoundMessage::Input { id, input, state } => {
                self.runtime.insert_input(id, input, state);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.runtime.display_api_handle().unwrap().request_redraw();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Forcefully drop the machine to stop it from being dropped on the audio thread and causing a panic
        self.runtime.maybe_machine.write().unwrap().take();

        // Save the config
        let environment_guard = self.runtime.environment.read().unwrap();
        let file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
        write_environment(file, &environment_guard).unwrap();
    }
}

fn setup_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let window_attributes = Window::default_attributes()
        .with_title("MultiEMU")
        .with_resizable(true)
        .with_transparent(false);
    Arc::new(event_loop.create_window(window_attributes).unwrap())
}
