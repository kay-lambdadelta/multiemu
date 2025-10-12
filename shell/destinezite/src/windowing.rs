use crate::{
    audio::CpalAudioRuntime,
    input::{
        gamepad::gamepad_task,
        keyboard::{KEYBOARD_ID, winit2key},
    },
};
use egui::RawInput;
use egui_winit::egui::ViewportId;
use multiemu_base::{
    environment::{ENVIRONMENT_LOCATION, Environment},
    graphics::GraphicsApi,
    input::{GamepadId, Input, InputState},
    platform::Platform,
    program::{ProgramMetadata, ProgramSpecification},
    utils::{MainThreadCallback, MainThreadExecutor},
};
use multiemu_frontend::{
    DisplayApiHandle, EguiPlatformIntegration, FrontendRuntime, GraphicsRuntime, MachineFactories,
    PlatformExt,
};
use nalgebra::Vector2;
use std::{
    any::Any,
    cell::OnceCell,
    fmt::Debug,
    fs::File,
    ops::Deref,
    sync::{Arc, Condvar, Mutex, RwLock},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
    },
    window::{Window, WindowId},
};

pub enum RuntimeBoundMessage {
    Input {
        id: GamepadId,
        input: Input,
        state: InputState,
    },
    ExecuteCallback {
        callback: MainThreadCallback,
        return_slot: Arc<Mutex<Option<Box<dyn Any + Send>>>>,
        condvar: Arc<Condvar>,
    },
}

impl Debug for RuntimeBoundMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeBoundMessage").finish()
    }
}

#[derive(Debug, Clone)]
/// Newtype for a winit window
pub struct WinitWindow(Arc<Window>);

impl WinitWindow {
    pub fn inner(&self) -> Arc<Window> {
        self.0.clone()
    }
}

impl HasDisplayHandle for WinitWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

impl HasWindowHandle for WinitWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl DisplayApiHandle for WinitWindow {
    fn dimensions(&self) -> nalgebra::Vector2<u32> {
        let size = self.0.inner_size();

        Vector2::new(size.width, size.height)
    }
}

pub struct WinitEguiPlatformIntegration {
    egui_winit: Option<egui_winit::State>,
    display_api_handle: WinitWindow,
}

impl Debug for WinitEguiPlatformIntegration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinitEguiPlatformIntegration").finish()
    }
}

impl EguiPlatformIntegration<WinitWindow> for WinitEguiPlatformIntegration {
    fn set_egui_context(&mut self, context: &egui::Context) {
        self.egui_winit = Some(egui_winit::State::new(
            context.clone(),
            ViewportId::ROOT,
            &self.display_api_handle,
            Some(self.display_api_handle.0.scale_factor() as f32),
            None,
            None,
        ));
    }

    fn gather_platform_specific_inputs(&mut self) -> RawInput {
        self.egui_winit
            .as_mut()
            .expect("egui_winit not initialized")
            .take_egui_input(&self.display_api_handle.0)
    }
}

#[derive(Debug)]
pub struct DesktopPlatform<
    G: GraphicsApi,
    GR: GraphicsRuntime<Self, DisplayApiHandle = WinitWindow>,
> {
    runtime: FrontendRuntime<Self>,
    display_api_handle: OnceCell<WinitWindow>,
    environment: Arc<RwLock<Environment>>,
}

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, DisplayApiHandle = WinitWindow>> Platform
    for DesktopPlatform<G, GR>
{
    type GraphicsApi = G;
    type MainThreadExecutor = WinitMainThreadExecutor;
}

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, DisplayApiHandle = WinitWindow>> PlatformExt
    for DesktopPlatform<G, GR>
{
    type GraphicsRuntime = GR;
    type AudioRuntime = CpalAudioRuntime<Self>;
    type EguiPlatformIntegration = WinitEguiPlatformIntegration;

    fn run(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramMetadata>,
        machine_factories: MachineFactories<Self>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::run_common(environment, program_manager, machine_factories, None)?;

        Ok(())
    }

    fn run_with_program(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramMetadata>,
        machine_factories: MachineFactories<Self>,
        program_specification: ProgramSpecification,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::run_common(
            environment,
            program_manager,
            machine_factories,
            Some(program_specification),
        )?;

        Ok(())
    }
}

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, DisplayApiHandle = WinitWindow>>
    ApplicationHandler<RuntimeBoundMessage> for DesktopPlatform<G, GR>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let display_api_handle = setup_window(event_loop);

        tracing::debug!("Scale factor: {}", display_api_handle.0.scale_factor());
        let egui_platform_integration = WinitEguiPlatformIntegration {
            egui_winit: None,
            display_api_handle: display_api_handle.clone(),
        };

        self.runtime
            .set_display_api_handle(display_api_handle.clone(), egui_platform_integration);

        self.display_api_handle.set(display_api_handle).unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let display_api_handle = self.display_api_handle.get().unwrap();

        let egui_winit = self.runtime.egui_platform_integration();
        let _ = egui_winit
            .egui_winit
            .as_mut()
            .unwrap()
            .on_window_event(&display_api_handle.0, &event);

        match event {
            WindowEvent::Focused(focused) => {
                self.runtime.focus_change(focused);
            }
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
                self.runtime.redraw();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeBoundMessage) {
        match event {
            RuntimeBoundMessage::Input { id, input, state } => {
                self.runtime.insert_input(id, input, state);
            }
            RuntimeBoundMessage::ExecuteCallback {
                callback,
                return_slot,
                condvar,
            } => {
                let result = callback();
                let mut return_slot_guard = return_slot.lock().unwrap();
                *return_slot_guard = Some(result);
                condvar.notify_one();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.display_api_handle.get().unwrap().0.has_focus() {
            self.display_api_handle.get().unwrap().0.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Forcefully drop the machine to stop it from being dropped on the audio thread and causing a panic
        self.runtime.maybe_machine().write().unwrap().take();

        // Save the config
        let environment_guard = self.environment.read().unwrap();
        let file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
        environment_guard.save(file).unwrap();
    }
}

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, DisplayApiHandle = WinitWindow>>
    DesktopPlatform<G, GR>
{
    fn run_common(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramMetadata>,
        machine_factories: MachineFactories<Self>,
        program_specification: Option<ProgramSpecification>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

        let main_thread_executor = Arc::new(WinitMainThreadExecutor {
            event_loop_proxy: event_loop.create_proxy(),
        });

        let runtime = if let Some(program_specification) = program_specification {
            FrontendRuntime::new_with_machine(
                environment.clone(),
                program_manager,
                machine_factories,
                main_thread_executor,
                program_specification,
            )
        } else {
            FrontendRuntime::new(
                environment.clone(),
                program_manager,
                machine_factories,
                main_thread_executor,
            )
        };

        let mut me = DesktopPlatform {
            runtime,
            display_api_handle: OnceCell::default(),
            environment,
        };

        event_loop.run_app(&mut me)?;

        Ok(())
    }
}

fn setup_window(event_loop: &ActiveEventLoop) -> WinitWindow {
    let window_attributes = Window::default_attributes()
        .with_title("MultiEMU")
        .with_resizable(true)
        .with_transparent(false);

    WinitWindow(Arc::new(
        event_loop.create_window(window_attributes).unwrap(),
    ))
}

#[derive(Debug)]
pub struct WinitMainThreadExecutor {
    event_loop_proxy: EventLoopProxy<RuntimeBoundMessage>,
}

impl MainThreadExecutor for WinitMainThreadExecutor {
    fn execute(&self, callback: MainThreadCallback) -> Box<dyn Any + Send> {
        let return_slot = Arc::new(Mutex::new(None));
        let condvar = Arc::new(Condvar::new());

        self.event_loop_proxy
            .send_event(RuntimeBoundMessage::ExecuteCallback {
                callback,
                return_slot: return_slot.clone(),
                condvar: condvar.clone(),
            })
            .unwrap();

        let mut return_slot_guard = return_slot.lock().unwrap();

        // Wait for our returned value
        return_slot_guard = condvar
            .wait_while(return_slot_guard, |return_slot_guard| {
                return_slot_guard.is_none()
            })
            .unwrap();

        return_slot_guard.take().unwrap()
    }
}
