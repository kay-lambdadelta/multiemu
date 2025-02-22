use super::keyboard::{KEYBOARD_ID, winit2key};
use super::main_loop::Message;
use super::{PlatformRuntime, WindowingContext};
use crate::rendering_backend::RenderingBackendState;
use crate::runtime::desktop::gamepad::gamepad_task;
use crate::runtime::desktop::main_loop::MainLoop;
use egui::ViewportId;
use multiemu_input::InputState;
use multiemu_machine::display::RenderBackend;
use std::sync::{Arc, Mutex};
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop, window::Window};

impl<R: RenderBackend, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>>
    ApplicationHandler for PlatformRuntime<RS>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // HACK: This will cause frequent crashes on mobile platforms
        if self.windowing.is_some() {
            panic!("Window already created");
        }

        let display_api_handle = setup_window(event_loop);
        let (message_channel_sender, message_channel_receiver) = crossbeam::channel::unbounded();
        let egui_context = egui::Context::default();
        let egui_winit = Arc::new(Mutex::new(egui_winit::State::new(
            egui_context.clone(),
            ViewportId::ROOT,
            &display_api_handle,
            Some(display_api_handle.scale_factor() as f32),
            None,
            None,
        )));
        let rom_manager = self.rom_manager.clone();
        let environment = self.environment.clone();

        {
            let display_api_handle = display_api_handle.clone();
            let egui_context = egui_context.clone();
            let egui_winit = egui_winit.clone();

            std::thread::Builder::new()
                .name("main_loop".to_string())
                .spawn(move || {
                    tracing::debug!("Starting up runtime thread");

                    let mut runtime = MainLoop::<R, RS>::new(
                        message_channel_receiver,
                        display_api_handle,
                        rom_manager,
                        environment,
                        egui_context,
                        egui_winit,
                    );

                    runtime.run();

                    tracing::debug!("Shutting down runtime thread");
                })
                .unwrap();
        }

        {
            let message_channel_sender = message_channel_sender.clone();

            std::thread::Builder::new()
                .name("gamepad".to_string())
                .spawn(move || {
                    tracing::debug!("Starting up gamepad thread");

                    gamepad_task(message_channel_sender);

                    tracing::debug!("Shutting down gamepad thread");
                })
                .unwrap();
        }

        if let Some((game_system, user_specified_roms)) = self.pending_machine.take() {
            message_channel_sender
                .send(Message::RunMachine {
                    game_system,
                    user_specified_roms,
                })
                .unwrap();
        }

        self.windowing = Some(WindowingContext {
            display_api_handle,
            egui_winit,
            runtime_channel: message_channel_sender,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let windowing = self.windowing.as_ref().unwrap();

        let mut egui_winit_guard = windowing.egui_winit.lock().unwrap();
        // We ignore the "consumed" event so we can process keys, the main loop knows itself when to send things to egui or not
        let _ = egui_winit_guard.on_window_event(&windowing.display_api_handle, &event);

        drop(egui_winit_guard);

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Window close requested");

                // Save the config on exit
                self.environment
                    .read()
                    .unwrap()
                    .save()
                    .expect("Failed to save config");

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

                if let Some(key) = winit2key(event.physical_key) {
                    let state = InputState::Digital(event.state.is_pressed());

                    windowing
                        .runtime_channel
                        .send(Message::Input {
                            id: KEYBOARD_ID,
                            input: key,
                            state,
                        })
                        .unwrap();
                }
            }
            _ => {}
        }
    }
}

fn setup_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let window_attributes = Window::default_attributes()
        .with_title("MultiEMU")
        .with_resizable(true)
        .with_transparent(false);
    Arc::new(event_loop.create_window(window_attributes).unwrap())
}
