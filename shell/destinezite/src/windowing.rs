use std::{borrow::Cow, collections::HashMap, fmt::Debug, fs::File, ops::Deref, sync::Arc};

use egui::RawInput;
use egui_winit::egui::ViewportId;
use fluxemu_frontend::{
    EguiWindowingIntegration, Frontend, GraphicsRuntime, MachineFactories, PlatformExt,
    WindowingHandle,
    environment::{ENVIRONMENT_LOCATION, Environment},
};
use fluxemu_runtime::{
    graphics::GraphicsApi,
    input::{
        GamepadInput, Input, InputState, RealGamepad, RealGamepadId, RealGamepadMetadata,
        keyboard::KeyboardInput,
    },
    platform::Platform,
    program::{ProgramManager, ProgramSpecification},
};
use gilrs::{EventType, Gilrs, GilrsBuilder};
use nalgebra::Vector2;
use strum::IntoEnumIterator;
use uuid::Uuid;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
    },
    window::{Theme, Window, WindowId},
};

use crate::{
    audio::CpalAudioRuntime,
    input::{
        gamepad::{gilrs_axis2input, gilrs_button2input},
        keyboard::{KEYBOARD_ID, winit2key},
    },
};

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

impl WindowingHandle for WinitWindow {
    fn physical_size(&self) -> nalgebra::Vector2<u32> {
        let size = self.0.inner_size();

        Vector2::new(size.width, size.height)
    }

    fn scale(&self) -> f64 {
        self.0.scale_factor()
    }
}

pub struct WinitEguiPlatformIntegration {
    egui_winit: Option<egui_winit::State>,
    display_api_handle: WinitWindow,
    theme: Theme,
}

impl Debug for WinitEguiPlatformIntegration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinitEguiPlatformIntegration").finish()
    }
}

impl EguiWindowingIntegration<WinitWindow> for WinitEguiPlatformIntegration {
    fn set_egui_context(&mut self, context: &egui::Context) {
        self.egui_winit = Some(egui_winit::State::new(
            context.clone(),
            ViewportId::ROOT,
            &self.display_api_handle,
            Some(self.display_api_handle.0.scale_factor() as f32),
            Some(self.theme),
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
pub struct DesktopPlatform<G: GraphicsApi, GR: GraphicsRuntime<Self, WindowingHandle = WinitWindow>>
{
    frontend: Frontend<Self>,
    display_api_handle: Option<WinitWindow>,
    gilrs_context: Gilrs,
    non_stable_controller_identification: HashMap<gilrs::GamepadId, Uuid>,
}

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, WindowingHandle = WinitWindow>> Platform
    for DesktopPlatform<G, GR>
{
    type GraphicsApi = G;
}

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, WindowingHandle = WinitWindow>> PlatformExt
    for DesktopPlatform<G, GR>
{
    type GraphicsRuntime = GR;
    type AudioRuntime = CpalAudioRuntime;
    type EguiWindowingIntegration = WinitEguiPlatformIntegration;

    fn run(
        environment: Environment,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<Self>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::run_common(environment, program_manager, machine_factories, None)?;

        Ok(())
    }

    fn run_with_program(
        environment: Environment,
        program_manager: Arc<ProgramManager>,
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

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, WindowingHandle = WinitWindow>> ApplicationHandler<()>
    for DesktopPlatform<G, GR>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let display_api_handle = setup_window(event_loop);

        tracing::info!("Scale factor: {}", display_api_handle.scale());
        let egui_platform_integration = WinitEguiPlatformIntegration {
            egui_winit: None,
            display_api_handle: display_api_handle.clone(),
            theme: event_loop.system_theme().unwrap_or(Theme::Dark),
        };

        self.frontend
            .set_windowing_handle(display_api_handle.clone(), egui_platform_integration);

        self.display_api_handle = Some(display_api_handle);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let display_api_handle = self.display_api_handle.as_ref().unwrap();

        let windowing_integration = self.frontend.get_windowing_integration().unwrap();

        let _ = windowing_integration
            .egui_winit
            .as_mut()
            .unwrap()
            .on_window_event(&display_api_handle.0, &event);

        match event {
            WindowEvent::Focused(focused) => {
                self.frontend.focus_change(focused);
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

                    self.frontend
                        .get_gamepad(KEYBOARD_ID)
                        .unwrap()
                        .set(input, state);
                }
            }
            WindowEvent::RedrawRequested => {
                while let Some(ev) = self.gilrs_context.next_event() {
                    let gilrs_gamepad = self.gilrs_context.gamepad(ev.id);

                    let gamepad_id = produce_id_for_gilrs_gamepad(
                        &mut self.non_stable_controller_identification,
                        ev.id,
                        gilrs_gamepad,
                    );

                    match ev.event {
                        EventType::Connected => {
                            let gamepad = RealGamepad::new(RealGamepadMetadata {
                                name: Cow::Owned(gilrs_gamepad.name().to_string()),
                                present_inputs: GamepadInput::iter().map(Input::Gamepad).collect(),
                            });

                            self.frontend.insert_gamepad(gamepad_id, gamepad);
                        }
                        EventType::AxisChanged(axis, value, _) => {
                            let gamepad = self.frontend.get_gamepad(gamepad_id).unwrap();

                            if let Some((input, state)) = gilrs_axis2input(axis, value) {
                                gamepad.set(input, state);
                            }
                        }
                        EventType::ButtonChanged(button, value, _) => {
                            let gamepad = self.frontend.get_gamepad(gamepad_id).unwrap();

                            if let Some(input) = gilrs_button2input(button) {
                                gamepad.set(input, InputState::Analog(value));
                            }
                        }
                        EventType::Disconnected => {
                            self.non_stable_controller_identification.remove(&ev.id);
                            self.frontend.remove_gamepad(gamepad_id);
                        }
                        _ => {}
                    }
                }

                self.frontend.redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.display_api_handle.as_ref().unwrap().0.has_focus() {
            self.display_api_handle.as_ref().unwrap().0.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let mut environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
        self.frontend
            .environment
            .save(&mut environment_file)
            .unwrap();
    }
}

impl<G: GraphicsApi, GR: GraphicsRuntime<Self, WindowingHandle = WinitWindow>>
    DesktopPlatform<G, GR>
{
    fn run_common(
        environment: Environment,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<Self>,
        program_specification: Option<ProgramSpecification>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::with_user_event().build()?;
        let gilrs_context = GilrsBuilder::new().build().unwrap();
        let mut non_stable_controller_identification = HashMap::new();

        let mut frontend = if let Some(program_specification) = program_specification {
            Frontend::new_with_machine(
                environment,
                program_manager,
                machine_factories,
                program_specification,
            )
        } else {
            Frontend::new(environment, program_manager, machine_factories)
        };

        for (gilrs_gamepad_id, gilrs_gamepad) in gilrs_context.gamepads() {
            let gamepad_id = produce_id_for_gilrs_gamepad(
                &mut non_stable_controller_identification,
                gilrs_gamepad_id,
                gilrs_gamepad,
            );

            let gamepad = RealGamepad::new(RealGamepadMetadata {
                name: Cow::Owned(gilrs_gamepad.name().to_string()),
                present_inputs: GamepadInput::iter().map(Input::Gamepad).collect(),
            });

            frontend.insert_gamepad(gamepad_id, gamepad);
        }

        let gamepad = RealGamepad::new(RealGamepadMetadata {
            name: Cow::Borrowed("Keyboard"),
            // We really can't make any assumptions about what the keyboard has so lets say "All of
            // them"
            present_inputs: KeyboardInput::iter().map(Input::Keyboard).collect(),
        });
        frontend.insert_gamepad(KEYBOARD_ID, gamepad.clone());

        let mut me = DesktopPlatform {
            frontend,
            display_api_handle: None,
            gilrs_context,
            non_stable_controller_identification,
        };

        event_loop.run_app(&mut me)?;

        Ok(())
    }
}

fn produce_id_for_gilrs_gamepad(
    non_stable_controller_identification: &mut HashMap<gilrs::GamepadId, Uuid>,
    gilrs_gamepad_id: gilrs::GamepadId,
    gilrs_gamepad: gilrs::Gamepad<'_>,
) -> RealGamepadId {
    let mut gamepad_id = Uuid::from_bytes(gilrs_gamepad.uuid());
    if gamepad_id == Uuid::nil() {
        gamepad_id = *non_stable_controller_identification
            .entry(gilrs_gamepad_id)
            .or_insert_with(|| {
                tracing::warn!(
                    "Gamepad {} is not giving us an ID, assigning it a arbitary one",
                    gamepad_id
                );

                Uuid::new_v4()
            });
    }
    RealGamepadId::new(gamepad_id.try_into().unwrap())
}

fn setup_window(event_loop: &ActiveEventLoop) -> WinitWindow {
    let window_attributes = Window::default_attributes()
        .with_title("FluxEMU")
        .with_resizable(true)
        .with_transparent(false)
        .with_decorations(true);

    WinitWindow(Arc::new(
        event_loop.create_window(window_attributes).unwrap(),
    ))
}
