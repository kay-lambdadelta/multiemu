use super::audio::CpalAudio;
use super::keyboard::{KEYBOARD_ID, winit2key};
use super::{RuntimeBoundMessage, setup_theme};
use crate::build_machine::build_machine;
use crate::gui::menu::{MenuState, UiOutput};
use crate::rendering_backend::RenderingBackendState;
use crate::runtime::Runtime;
use crate::runtime::desktop::gamepad::gamepad_task;
use egui::ViewportId;
use multiemu_config::Environment;
use multiemu_input::{GamepadId, Input, InputState};
use multiemu_machine::Machine;
use multiemu_machine::builder::display::BackendSpecificData;
use multiemu_machine::display::backend::{ContextExtensionSpecification, RenderBackend};
use multiemu_machine::display::shader::ShaderCache;
use multiemu_machine::input::VirtualGamepadId;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use multiemu_rom::system::GameSystem;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::WindowId;
use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop, window::Window};

enum RuntimeMode<R: RenderBackend> {
    Idle,
    Pending {
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    },
    Running {
        machine: Machine<R>,
    },
}

pub struct PlatformRuntime<RS: RenderingBackendState> {
    windowing: Option<WindowingContext<RS>>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    shader_cache: Arc<ShaderCache>,
    egui_context: egui::Context,
    menu_state: MenuState,
    audio: CpalAudio,
    gamepad_mapping: HashMap<GamepadId, VirtualGamepadId>,
    mode: RuntimeMode<RS::RenderBackend>,
    gui_active: bool,
    event_loop: Option<EventLoop<RuntimeBoundMessage>>,
    previous_frame_render_time: Duration,
    previous_frame_time: Duration,
}

impl<R: RenderBackend, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>>
    Runtime<R> for PlatformRuntime<RS>
{
    fn new(
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let egui_context = egui::Context::default();
        let menu_state = MenuState::new(environment.clone(), rom_manager.clone());

        setup_theme(&egui_context);

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

        let me = Self {
            windowing: None,
            rom_manager,
            environment,
            shader_cache: Arc::new(ShaderCache::new(12)),
            egui_context,
            menu_state,
            audio: CpalAudio::default(),
            gamepad_mapping: HashMap::new(),
            mode: RuntimeMode::Idle,
            gui_active: true,
            event_loop: Some(event_loop),
            previous_frame_render_time: Duration::from_millis(16),
            previous_frame_time: Duration::from_millis(16),
        };

        Ok(me)
    }

    fn launch_gui(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.event_loop.take().unwrap().run_app(self)?;

        Ok(())
    }

    fn launch_game(
        &mut self,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.mode = RuntimeMode::Pending {
            game_system,
            user_specified_roms,
        };

        self.event_loop.take().unwrap().run_app(self)?;

        Ok(())
    }
}

pub struct WindowingContext<RS: RenderingBackendState> {
    egui_winit: egui_winit::State,
    display_api_handle: RS::DisplayApiHandle,
    state: RS,
}

impl<R: RenderBackend, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>>
    ApplicationHandler<RuntimeBoundMessage> for PlatformRuntime<RS>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // HACK: This will cause frequent crashes on mobile platforms
        if self.windowing.is_some() {
            panic!("Window already created");
        }

        let display_api_handle = setup_window(event_loop);

        tracing::debug!("Scale factor: {}", display_api_handle.scale_factor());
        let egui_winit = egui_winit::State::new(
            self.egui_context.clone(),
            ViewportId::ROOT,
            &display_api_handle,
            Some(display_api_handle.scale_factor() as f32),
            None,
            None,
        );
        let environment = self.environment.clone();

        let state = match std::mem::replace(&mut self.mode, RuntimeMode::Idle) {
            RuntimeMode::Idle => RS::new(
                display_api_handle.clone(),
                environment.clone(),
                self.shader_cache.clone(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
            )
            .unwrap(),
            RuntimeMode::Pending {
                game_system,
                user_specified_roms,
            } => {
                tracing::info!("Starting up machine for {}", game_system);

                self.setup_render_backend_and_machine(
                    display_api_handle.clone(),
                    environment.clone(),
                    game_system,
                    user_specified_roms,
                )
            }
            RuntimeMode::Running { .. } => {
                unreachable!("Cannot recreate window while machine is active");
            }
        };

        self.windowing = Some(WindowingContext {
            display_api_handle,
            egui_winit,
            state,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let windowing = self.windowing.as_mut().unwrap();

        let _ = windowing
            .egui_winit
            .on_window_event(&windowing.display_api_handle, &event);

        if !matches!(self.mode, RuntimeMode::Running { .. }) {
            self.gui_active = true;
        }

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
            WindowEvent::Resized(_) => {
                windowing.state.surface_resized();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic,
            } => {
                if is_synthetic {
                    return;
                }

                if !self.gui_active {
                    let RuntimeMode::Running { machine } = &self.mode else {
                        unreachable!()
                    };

                    if let Some(input) = winit2key(event.physical_key) {
                        let state = InputState::Digital(event.state.is_pressed());

                        insert_input(
                            KEYBOARD_ID,
                            input,
                            state,
                            machine,
                            &self.gamepad_mapping,
                            &self.environment,
                        );
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if self.gui_active {
                    // We put the ui output like this so multipassing egui gui building works
                    let mut ui_output = None;
                    let full_output = self.egui_context.clone().run(
                        windowing
                            .egui_winit
                            .take_egui_input(&windowing.display_api_handle),
                        |context| {
                            ui_output = ui_output.take().or(self.menu_state.run_menu(context));
                        },
                    );

                    match ui_output {
                        None => {}
                        Some(UiOutput::Resume) => {
                            self.gui_active = false;
                        }
                        Some(UiOutput::OpenGame { rom_id }) => {
                            let database_transaction =
                                self.rom_manager.rom_information.begin_read().unwrap();
                            let database_table = database_transaction
                                .open_multimap_table(ROM_INFORMATION_TABLE)
                                .unwrap();
                            let rom_info = database_table
                                .get(&rom_id)
                                .unwrap()
                                .next()
                                .unwrap()
                                .unwrap()
                                .value();

                            let WindowingContext {
                                egui_winit,
                                display_api_handle,
                                state,
                            } = self.windowing.take().unwrap();
                            drop(state);

                            let state = self.setup_render_backend_and_machine(
                                display_api_handle.clone(),
                                self.environment.clone(),
                                rom_info.system,
                                vec![rom_id],
                            );

                            self.windowing = Some(WindowingContext {
                                display_api_handle,
                                egui_winit,
                                state,
                            });
                            return;
                        }
                    }

                    windowing.state.redraw_menu(&self.egui_context, full_output);
                } else {
                    let start_period = Instant::now();

                    let RuntimeMode::Running { machine } = &mut self.mode else {
                        unreachable!()
                    };

                    machine.run(self.previous_frame_time, self.previous_frame_render_time);

                    let start_frame_render_period = Instant::now();
                    windowing.state.redraw(machine);
                    let frame_render_duration = start_frame_render_period.elapsed();
                    let frame_duration = start_period.elapsed();

                    self.previous_frame_time = frame_duration;
                    self.previous_frame_render_time = frame_render_duration;
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeBoundMessage) {
        match event {
            RuntimeBoundMessage::Input { id, input, state } => {
                match &mut self.mode {
                    RuntimeMode::Idle => {}
                    RuntimeMode::Pending { .. } => {}
                    RuntimeMode::Running { machine } => {
                        if !self.gui_active {
                            // Translate input and feed into machine if possible
                            insert_input(
                                id,
                                input,
                                state,
                                machine,
                                &self.gamepad_mapping,
                                &self.environment,
                            );
                        }
                    }
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.windowing
            .as_ref()
            .unwrap()
            .display_api_handle
            .request_redraw();
    }
}

impl<R: RenderBackend, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>>
    PlatformRuntime<RS>
{
    fn setup_render_backend_and_machine(
        &mut self,
        display_api_handle: RS::DisplayApiHandle,
        environment: Arc<RwLock<Environment>>,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) -> RS {
        let machine_builder = build_machine(
            game_system,
            user_specified_roms,
            self.rom_manager.clone(),
            self.environment.clone(),
            self.shader_cache.clone(),
        );

        let mut preferred_extensions = R::ContextExtensionSpecification::default();
        let mut required_extensions = R::ContextExtensionSpecification::default();

        for (_component_id, component) in machine_builder.component_metadata.iter() {
            if let Some(display) = &component.display {
                let backend_specific_data = display
                    .backend_specific_data
                    .get(&TypeId::of::<R>())
                    .and_then(|data| data.downcast_ref::<BackendSpecificData<R>>())
                    .expect("Could not find display backend data for component");

                preferred_extensions = preferred_extensions
                    .combine(backend_specific_data.preferred_extensions.clone());
                required_extensions =
                    required_extensions.combine(backend_specific_data.required_extensions.clone());
            }
        }

        let render_backend_state = RS::new(
            display_api_handle.clone(),
            environment.clone(),
            self.shader_cache.clone(),
            preferred_extensions,
            required_extensions,
        )
        .unwrap();

        let machine =
            machine_builder.build::<R>(render_backend_state.component_initialization_data());

        // HACK: Map the keyboard to the first gamepad
        if let Some(virtual_gamepad_id) = machine.virtual_gamepads.keys().next().copied() {
            self.gamepad_mapping.insert(KEYBOARD_ID, virtual_gamepad_id);
        }

        self.mode = RuntimeMode::Running { machine };
        self.gui_active = false;

        render_backend_state
    }
}

fn setup_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let window_attributes = Window::default_attributes()
        .with_title("MultiEMU")
        .with_resizable(true)
        .with_transparent(false);
    Arc::new(event_loop.create_window(window_attributes).unwrap())
}

fn insert_input<R: RenderBackend>(
    id: GamepadId,
    input: Input,
    state: InputState,
    machine: &Machine<R>,
    gamepad_mapping: &HashMap<GamepadId, VirtualGamepadId>,
    environment: &RwLock<Environment>,
) {
    if let Some(virtual_id) = gamepad_mapping.get(&id) {
        let environment_guard = environment.read().unwrap();

        if let Some(virtual_gamepad) = machine.virtual_gamepads.get(virtual_id) {
            if let Some(transformed_input) = environment_guard
                .gamepad_configs
                .get(&machine.game_system)
                .and_then(|gamepad_types| gamepad_types.get(&virtual_gamepad.name()))
                .and_then(|gamepad_transformer| gamepad_transformer.get(&input))
            {
                virtual_gamepad.set(*transformed_input, state);
            }
        }
    }
}
