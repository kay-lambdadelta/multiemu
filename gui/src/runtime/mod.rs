use egui::{FontFamily, RawInput, TextStyle, TextWrapMode};
use multiemu_config::{Environment, input::Hotkey};
use multiemu_input::{GamepadId, Input, InputState};
use multiemu_machine::{
    Machine,
    display::{RenderExtensions, backend::RenderApi, shader::ShaderCache},
    input::VirtualGamepadId,
};
use multiemu_rom::{
    id::RomId,
    manager::{ROM_INFORMATION_TABLE, RomManager},
    system::GameSystem,
};
use nalgebra::Vector2;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

#[cfg(platform_desktop)]
pub mod desktop;
#[cfg(platform_desktop)]
pub use desktop::renderer::software::SoftwareRenderingRuntime;

#[cfg(platform_3ds)]
pub mod nintendo_3ds;
#[cfg(platform_3ds)]
pub use nintendo_3ds::renderer::software::SoftwareRenderingRuntime;

use crate::{
    build_machine::MachineFactories,
    gui::menu::{MenuState, UiOutput},
    rendering_backend::{DisplayApiHandle, RenderingBackendState},
};

fn setup_theme(egui_context: &egui::Context) {
    egui_context.style_mut(|style| {
        // Wrapping breaks tables
        style.wrap_mode = Some(TextWrapMode::Extend);

        style.text_styles.insert(
            TextStyle::Body,
            egui::FontId::new(18.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Button,
            egui::FontId::new(20.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            egui::FontId::new(24.0, FontFamily::Proportional),
        );
    });
}

/// A runtime for a given platform
pub trait Platform<RS: RenderingBackendState> {
    fn run(runtime: Runtime<RS>) -> Result<(), Box<dyn std::error::Error>>;
}

enum MaybeMachine<R: RenderApi> {
    Machine(Machine<R>),
    PendingMachine {
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    },
}

pub struct WindowingContext<RS: RenderingBackendState> {
    display_api_handle: RS::DisplayApiHandle,
    state: RS,
}

enum RuntimeMode<R: RenderApi> {
    Machine(Machine<R>),
    Gui(Option<MaybeMachine<R>>),
}

impl<R: RenderApi> Debug for RuntimeMode<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeMode::Machine(_) => write!(f, "RuntimeMode::Machine"),
            RuntimeMode::Gui(_) => write!(f, "RuntimeMode::Gui"),
        }
    }
}

pub struct Runtime<RS: RenderingBackendState> {
    mode: RuntimeMode<RS::RenderApi>,
    gamepad_mapping: HashMap<GamepadId, VirtualGamepadId>,
    pub environment: Arc<RwLock<Environment>>,
    pub rom_manager: Arc<RomManager>,
    pub egui_context: egui::Context,
    windowing_context: Option<WindowingContext<RS>>,
    shader_cache: ShaderCache,
    menu_state: MenuState,
    previous_frame_render_time: Duration,
    previous_frame_time: Duration,
    previous_window_size: Vector2<u16>,
    currently_key_states: HashMap<GamepadId, HashMap<Input, InputState>>,
    was_egui_context_reset: bool,
    machine_factories: MachineFactories<RS::RenderApi>,
}

impl<RS: RenderingBackendState> Runtime<RS> {
    pub fn new(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomManager>,
        machine_factories: MachineFactories<RS::RenderApi>,
    ) -> Self {
        let egui_context = egui::Context::default();
        setup_theme(&egui_context);
        let menu_state = MenuState::new(environment.clone(), rom_manager.clone());
        let gamepad_mapping = HashMap::new();
        let mode = RuntimeMode::Gui(None);
        let shader_cache = ShaderCache::new(environment.clone());

        Self {
            mode,
            gamepad_mapping,
            environment,
            rom_manager,
            shader_cache,
            egui_context,
            menu_state,
            windowing_context: None,
            previous_frame_render_time: Duration::ZERO,
            previous_frame_time: Duration::ZERO,
            previous_window_size: Vector2::zeros(),
            currently_key_states: HashMap::new(),
            was_egui_context_reset: false,
            machine_factories,
        }
    }

    pub fn was_egui_context_reset(&mut self) -> bool {
        std::mem::replace(&mut self.was_egui_context_reset, false)
    }

    /// Late initialization of the display API handle
    pub fn set_display_api_handle(&mut self, display_api_handle: RS::DisplayApiHandle) {
        match std::mem::replace(&mut self.mode, RuntimeMode::Gui(None)) {
            RuntimeMode::Machine(_) | RuntimeMode::Gui(Some(MaybeMachine::Machine(_))) => {
                unreachable!("Display API handle should be set before the machine is created")
            }
            RuntimeMode::Gui(None) => {
                self.egui_context = egui::Context::default();
                setup_theme(&self.egui_context);
                self.was_egui_context_reset = true;

                let render_backend_state = RS::new(
                    display_api_handle.clone(),
                    self.environment.clone(),
                    self.shader_cache.clone(),
                    RenderExtensions::default(),
                )
                .unwrap();

                let windowing = WindowingContext {
                    display_api_handle,
                    state: render_backend_state,
                };
                self.windowing_context = Some(windowing);
            }
            RuntimeMode::Gui(Some(MaybeMachine::PendingMachine {
                game_system,
                user_specified_roms,
            })) => {
                self.setup_runtime_for_new_machine(
                    display_api_handle,
                    game_system,
                    user_specified_roms,
                );
            }
        }
    }

    fn setup_runtime_for_new_machine(
        &mut self,
        display_api_handle: <RS as RenderingBackendState>::DisplayApiHandle,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) {
        let machine_builder = self.machine_factories.construct_machine(
            game_system,
            user_specified_roms,
            self.rom_manager.clone(),
            self.environment.clone(),
            self.shader_cache.clone(),
        );

        // Drop old machine otherwise it will segfault when we try to use the new vulkan context
        self.mode = RuntimeMode::Gui(None);

        self.egui_context = egui::Context::default();
        setup_theme(&self.egui_context);
        self.was_egui_context_reset = true;

        let render_extensions = machine_builder.render_extensions();

        let render_backend_state = RS::new(
            display_api_handle.clone(),
            self.environment.clone(),
            self.shader_cache.clone(),
            render_extensions,
        )
        .unwrap();

        let machine = machine_builder.build(render_backend_state.component_initialization_data());

        let windowing = WindowingContext {
            display_api_handle,
            state: render_backend_state,
        };
        self.windowing_context = Some(windowing);

        if let Some((virtual_gamepad, _)) = machine.virtual_gamepads.iter().next() {
            self.gamepad_mapping
                .insert(GamepadId::PLATFORM_RESERVED, *virtual_gamepad);
        }

        self.mode = RuntimeMode::Machine(machine);
    }

    pub fn display_api_handle(&self) -> Option<RS::DisplayApiHandle> {
        self.windowing_context
            .as_ref()
            .map(|windowing| windowing.display_api_handle.clone())
    }

    pub fn new_with_machine(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomManager>,
        machine_factories: MachineFactories<RS::RenderApi>,

        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) -> Self {
        let mut me = Self::new(environment.clone(), rom_manager.clone(), machine_factories);

        me.mode = RuntimeMode::Gui(Some(MaybeMachine::PendingMachine {
            game_system,
            user_specified_roms,
        }));

        me
    }

    pub fn insert_input(&mut self, id: GamepadId, input: Input, state: InputState) {
        self.currently_key_states
            .entry(id)
            .or_default()
            .insert(input, state);

        // check for hotkeys

        let enviroment_guard = self.environment.read().unwrap();

        for (keys_to_press, action) in enviroment_guard.hotkeys.iter() {
            if keys_to_press.iter().all(|key| {
                self.currently_key_states
                    .get(&id)
                    .and_then(|map| map.get(key))
                    .map(|state| state.as_digital(None))
                    .unwrap_or(false)
            }) {
                tracing::debug!("Hotkey pressed: {:?}", action);

                match action {
                    Hotkey::ToggleMenu => {
                        match std::mem::replace(&mut self.mode, RuntimeMode::Gui(None)) {
                            RuntimeMode::Machine(machine) => {
                                self.mode = RuntimeMode::Gui(Some(MaybeMachine::Machine(machine)));
                            }
                            RuntimeMode::Gui(Some(MaybeMachine::Machine(machine))) => {
                                self.mode = RuntimeMode::Machine(machine);
                            }
                            RuntimeMode::Gui(Some(MaybeMachine::PendingMachine {
                                game_system,
                                user_specified_roms,
                            })) => {
                                self.mode = RuntimeMode::Gui(Some(MaybeMachine::PendingMachine {
                                    game_system,
                                    user_specified_roms,
                                }));
                            }
                            _ => {}
                        }
                    }
                    Hotkey::FastForward => todo!(),
                    Hotkey::LoadSnapshot => todo!(),
                    Hotkey::SaveSnapshot => todo!(),
                }
            }
        }

        match &self.mode {
            RuntimeMode::Machine(machine) => {
                if let Some(virtual_id) = self.gamepad_mapping.get(&id) {
                    if let Some(virtual_gamepad) = machine.virtual_gamepads.get(virtual_id) {
                        if let Some(transformed_input) = enviroment_guard
                            .gamepad_configs
                            .get(&machine.game_system)
                            .and_then(|gamepad_types| gamepad_types.get(&virtual_gamepad.name()))
                            .and_then(|gamepad_transformer| gamepad_transformer.get(&input))
                        {
                            tracing::debug!(
                                "Transformed input: {:?} -> {:?} to state {:?}",
                                input,
                                transformed_input,
                                state
                            );

                            virtual_gamepad.set(*transformed_input, state);
                        }
                    }
                }
            }
            RuntimeMode::Gui { .. } => {}
        }
    }

    pub fn redraw(&mut self, custom_gui_input: Option<RawInput>) {
        let windowing = self.windowing_context.as_mut().unwrap();

        if self.previous_window_size != windowing.display_api_handle.dimensions() {
            windowing.state.surface_resized();
            self.previous_window_size = windowing.display_api_handle.dimensions();
        }

        match &mut self.mode {
            RuntimeMode::Machine(machine) => {
                let start_period = Instant::now();

                let start_schedule_period = std::time::Instant::now();
                machine.scheduler.run();
                let elapsed = start_schedule_period.elapsed();
                let scheduler_alloted_time =
                    self.previous_frame_time - self.previous_frame_render_time;

                match elapsed.cmp(&scheduler_alloted_time) {
                    std::cmp::Ordering::Less => {
                        machine.scheduler.speed_up();
                    }
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Greater => {
                        machine.scheduler.slow_down();
                    }
                }

                let start_frame_render_period = Instant::now();
                windowing.state.redraw(machine);
                let frame_render_duration = start_frame_render_period.elapsed();
                let frame_duration = start_period.elapsed();

                self.previous_frame_time = frame_duration;
                self.previous_frame_render_time = frame_render_duration;
            }
            RuntimeMode::Gui(maybe_machine) => {
                let mut ui_output = None;
                let machine = match maybe_machine {
                    Some(MaybeMachine::Machine(machine)) => Some(machine),
                    Some(MaybeMachine::PendingMachine { .. }) => None,
                    None => None,
                }
                .map(|machine| &*machine);

                let full_output = self.egui_context.clone().run(
                    custom_gui_input.unwrap_or_default(),
                    |context| {
                        ui_output = ui_output
                            .take()
                            .or(self.menu_state.run_menu(context, machine));
                    },
                );

                match ui_output {
                    None => {}
                    Some(UiOutput::Resume) => {
                        match std::mem::replace(&mut self.mode, RuntimeMode::Gui(None)) {
                            RuntimeMode::Gui(Some(MaybeMachine::Machine(machine))) => {
                                self.mode = RuntimeMode::Machine(machine);
                            }
                            RuntimeMode::Gui(Some(MaybeMachine::PendingMachine {
                                game_system,
                                user_specified_roms,
                            })) => {
                                self.mode = RuntimeMode::Gui(Some(MaybeMachine::PendingMachine {
                                    game_system,
                                    user_specified_roms,
                                }));
                            }
                            _ => {}
                        }
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
                            display_api_handle,
                            state,
                        } = self.windowing_context.take().unwrap();
                        drop(state);

                        self.setup_runtime_for_new_machine(
                            display_api_handle,
                            rom_info.system,
                            vec![rom_id],
                        );
                        return;
                    }
                }

                self.windowing_context
                    .as_mut()
                    .unwrap()
                    .state
                    .redraw_menu(&self.egui_context, full_output);
            }
        }
    }
}
