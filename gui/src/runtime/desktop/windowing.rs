use super::PlatformRuntime;
use crate::rendering_backend::RenderingBackendState;
use crate::runtime::main_loop::MainLoop;
use multiemu_input::GamepadId;
use multiemu_machine::display::RenderBackend;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop, window::Window};

const KEYBOARD_GAMEPAD_ID: GamepadId = 0;

impl<
        R: RenderBackend,
        RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>,
    > ApplicationHandler for PlatformRuntime<RS>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // HACK: This will cause frequent crashes on mobile platforms
        if self.display_api_handle.is_some() {
            panic!("Window already created");
        }

        let display_api_handle = setup_window(event_loop);
        self.display_api_handle.insert(display_api_handle.clone());

        let (message_channel_sender, message_channel_receiver) = crossbeam::channel::unbounded();
        let rom_manager = self.rom_manager.clone();
        let environment = self.environment.clone();

        std::thread::spawn(|| {
            let mut runtime = MainLoop::<R, RS>::new(
                message_channel_receiver,
                display_api_handle,
                rom_manager,
                environment,
            );

            runtime.run();
        });

        self.runtime_channel.insert(message_channel_sender);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
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
