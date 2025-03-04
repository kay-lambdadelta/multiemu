use super::keyboard::{KEYBOARD_ID, winit2key};
use super::{PlatformRuntime, RuntimeBoundMessage, RuntimeMode};
use crate::build_machine::build_machine;
use crate::gui::menu::UiOutput;
use crate::rendering_backend::RenderingBackendState;
use egui::ViewportId;
use multiemu_config::Environment;
use multiemu_input::{GamepadId, Input, InputState};
use multiemu_machine::Machine;
use multiemu_machine::builder::display::BackendSpecificData;
use multiemu_machine::display::{ContextExtensionSpecification, RenderBackend};
use multiemu_machine::input::VirtualGamepadId;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::{LoadedRomLocation, ROM_INFORMATION_TABLE};
use multiemu_rom::system::GameSystem;
use std::any::TypeId;
use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, RwLock};
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop, window::Window};

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
        let egui_winit = egui_winit::State::new(
            self.egui_context.clone(),
            ViewportId::ROOT,
            &display_api_handle,
            Some(display_api_handle.scale_factor() as f32),
            None,
            None,
        );
        let rom_manager = self.rom_manager.clone();
        let environment = self.environment.clone();

        let state = match std::mem::replace(&mut self.mode, RuntimeMode::Idle) {
            RuntimeMode::Idle => RS::new(
                display_api_handle.clone(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
                environment.clone(),
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

                        if let Some(virtual_id) = self.gamepad_mapping.get(&KEYBOARD_ID) {
                            let environment_guard = self.environment.read().unwrap();

                            if let Some(virtual_gamepad) = machine.virtual_gamepads.get(virtual_id)
                            {
                                if let Some(transformed_input) = environment_guard
                                    .gamepad_configs
                                    .get(&machine.game_system)
                                    .and_then(|gamepad_types| {
                                        gamepad_types.get(&virtual_gamepad.name())
                                    })
                                    .and_then(|gamepad_transformer| gamepad_transformer.get(&input))
                                {
                                    virtual_gamepad.set(*transformed_input, state);
                                }
                            }
                        }
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
                            ui_output = ui_output
                                .take()
                                .or(self.menu_state.run_menu(context, &self.rom_manager));
                        },
                    );

                    match ui_output {
                        None => {}
                        Some(UiOutput::Resume) => {
                            self.gui_active = false;
                        }
                        Some(UiOutput::OpenGame { path }) => {
                            tracing::info!("Opening ROM at {}", path.display());

                            let mut rom_file = File::open(&path).unwrap();
                            let rom_id = RomId::from_read(&mut rom_file);

                            let database_transaction =
                                self.rom_manager.rom_information.begin_read().unwrap();
                            let database_table = database_transaction
                                .open_table(ROM_INFORMATION_TABLE)
                                .unwrap();

                            // Try to figure out what kind of game is this
                            if let Some(game_system) = database_table
                                .get(rom_id)
                                .unwrap()
                                .map(|info| info.value().system)
                                .or_else(|| GameSystem::guess(&path))
                            {
                                self.rom_manager
                                    .loaded_roms
                                    .insert(rom_id, LoadedRomLocation::External(path.clone()))
                                    .unwrap();

                                let windowing = self.windowing.take().unwrap();
                                let state = self.setup_render_backend_and_machine(
                                    windowing.display_api_handle.clone(),
                                    self.environment.clone(),
                                    game_system,
                                    vec![rom_id],
                                );

                                self.windowing = Some(WindowingContext {
                                    display_api_handle: windowing.display_api_handle,
                                    egui_winit: windowing.egui_winit,
                                    state,
                                });
                                return;
                            } else {
                                tracing::error!("Could not identify ROM at {}", path.display());
                            }
                        }
                    }

                    windowing.state.redraw_menu(&self.egui_context, full_output);
                } else {
                    let RuntimeMode::Running { machine } = &mut self.mode else {
                        unreachable!()
                    };

                    self.timing_tracker.machine_main_cycle_starting();
                    machine.scheduler.run();
                    windowing.state.redraw(machine);
                    let time_taken = self.timing_tracker.machine_main_cycle_ending();

                    match time_taken.cmp(&self.timing_tracker.average_timings()) {
                        std::cmp::Ordering::Less => {
                            machine.scheduler.speed_up();
                        }
                        std::cmp::Ordering::Greater => {
                            machine.scheduler.slow_down();
                        }
                        std::cmp::Ordering::Equal => {}
                    }
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
            preferred_extensions,
            required_extensions,
            environment.clone(),
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
    machine: &mut Machine<R>,
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
