use crate::{
    build_machine::MachineFactories,
    gui::menu::{MenuState, UiOutput},
    rendering_backend::{DisplayApiHandle, RenderingBackendState},
    runtime::{AudioRuntime, MaybeMachine},
};
use egui::{FontFamily, RawInput, TextStyle, TextWrapMode};
use multiemu_config::{input::{Hotkey, DEFAULT_HOTKEYS}, Environment};
use multiemu_graphics::GraphicsContextExtensions;
use multiemu_input::{GamepadId, Input, InputState};
use multiemu_rom::{
    id::RomId,
    manager::{ROM_INFORMATION_TABLE, RomManager},
    system::GameSystem,
};
use multiemu_runtime::input::VirtualGamepadId;
use nalgebra::Vector2;
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::Deref,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
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

#[derive(Debug)]
pub struct WindowingContext<RS: RenderingBackendState> {
    display_api_handle: RS::DisplayApiHandle,
    state: RS,
}

#[derive(Debug)]
enum RuntimeMode {
    Machine,
    Gui,
}

#[derive(Debug)]
pub struct MainRuntime<RS: RenderingBackendState, AR: AudioRuntime> {
    /// If we are on the menu or in a game
    mode: RuntimeMode,
    /// What real gamepads connect to what
    gamepad_mapping: HashMap<GamepadId, VirtualGamepadId>,
    pub rom_manager: Arc<RomManager>,
    pub environment: Arc<RwLock<Environment>>,
    pub egui_context: egui::Context,
    /// Our windowing and rendering context
    windowing_context: Option<WindowingContext<RS>>,
    /// Bits and pieces of persistant menu state
    menu_state: MenuState,
    previous_frame_render_time: Duration,
    previous_frame_time: Duration,
    previous_window_size: Vector2<u32>,
    current_key_states: HashMap<GamepadId, HashMap<Input, InputState>>,
    /// HACK: Flag to tell the driver runtime if it needs to swap its egui integration out
    was_egui_context_reset: bool,
    machine_factories: MachineFactories<RS::GraphicsApi>,
    /// Our beloved simulator itself
    pub maybe_machine: Arc<MaybeMachine>,
    /// Data that the machine simulator needs when it's possible to build it
    pending_machine_resources: Option<(GameSystem, Vec<RomId>)>,
    audio_runtime: AR,
}

impl<RS: RenderingBackendState, AR: AudioRuntime> MainRuntime<RS, AR> {
    pub fn new(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomManager>,
        machine_factories: MachineFactories<RS::GraphicsApi>,
    ) -> Self {
        let maybe_machine: Arc<MaybeMachine> = Arc::default();
        let egui_context = egui::Context::default();
        setup_theme(&egui_context);
        let menu_state = MenuState::new(environment.clone(), rom_manager.clone());
        let gamepad_mapping = HashMap::new();
        let mode = RuntimeMode::Gui;
        let audio_runtime = AR::new(maybe_machine.clone());

        // Make sure we have some hotkeys at least
        let mut environment_guard = environment.write().unwrap();
        if environment_guard.hotkeys.is_empty() {
            environment_guard.hotkeys = DEFAULT_HOTKEYS.clone();
        }
        drop(environment_guard);

        audio_runtime.play();

        Self {
            mode,
            gamepad_mapping,
            environment,
            rom_manager,
            egui_context,
            menu_state,
            windowing_context: None,
            previous_frame_render_time: Duration::ZERO,
            previous_frame_time: Duration::ZERO,
            previous_window_size: Vector2::zeros(),
            current_key_states: HashMap::new(),
            was_egui_context_reset: false,
            machine_factories,
            maybe_machine: maybe_machine.clone(),
            pending_machine_resources: None,
            audio_runtime,
        }
    }

    pub fn was_egui_context_reset(&mut self) -> bool {
        std::mem::replace(&mut self.was_egui_context_reset, false)
    }

    /// Late initialization of the display API handle
    pub fn set_display_api_handle(&mut self, display_api_handle: RS::DisplayApiHandle) {
        let maybe_machine_guard = self.maybe_machine.read().unwrap();

        match maybe_machine_guard.deref() {
            Some(_) => {
                unreachable!("Cannot reinit machine!")
            }
            None => match self.mode {
                RuntimeMode::Machine => {
                    let (game_system, user_specified_roms) =
                        self.pending_machine_resources.take().unwrap();

                    drop(maybe_machine_guard);
                    self.setup_runtime_for_new_machine(
                        display_api_handle,
                        game_system,
                        user_specified_roms,
                    );
                }
                RuntimeMode::Gui => {
                    self.egui_context = egui::Context::default();
                    setup_theme(&self.egui_context);
                    self.was_egui_context_reset = true;

                    let render_backend_state = RS::new(
                        display_api_handle.clone(),
                        GraphicsContextExtensions::default(),
                        self.environment.clone(),
                    )
                    .unwrap();

                    let windowing = WindowingContext {
                        display_api_handle,
                        state: render_backend_state,
                    };
                    self.windowing_context = Some(windowing);
                }
            },
        }
    }

    fn setup_runtime_for_new_machine(
        &mut self,
        display_api_handle: <RS as RenderingBackendState>::DisplayApiHandle,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) {
        let mut maybe_machine_guard = self.maybe_machine.write().unwrap();
        // Drop old machine otherwise it will segfault when we try to use the new video context
        // I dunno how we could prevent this unsafety but beware future programmers
        maybe_machine_guard.take();

        let machine_builder = self.machine_factories.construct_machine(
            game_system,
            user_specified_roms,
            self.rom_manager.clone(),
            self.audio_runtime.sample_rate(),
        );

        self.egui_context = egui::Context::default();
        setup_theme(&self.egui_context);
        self.was_egui_context_reset = true;

        let render_extensions = machine_builder.render_extensions();

        let render_backend_state = RS::new(
            display_api_handle.clone(),
            render_extensions,
            self.environment.clone(),
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

        // Try to give the environment default bindings

        let mut environment_guard = self.environment.write().unwrap();

        // Make sure default mappings exist
        for virtual_gamepad in machine.virtual_gamepads.values() {
            environment_guard
                .gamepad_configs
                .entry(machine.game_system)
                .or_default()
                .entry(virtual_gamepad.name())
                .or_insert(
                    virtual_gamepad
                        .metadata()
                        .default_bindings
                        .clone()
                        .into_iter()
                        .collect(),
                );
        }

        self.mode = RuntimeMode::Machine;
        maybe_machine_guard.replace(machine);
    }

    pub fn display_api_handle(&self) -> Option<RS::DisplayApiHandle> {
        self.windowing_context
            .as_ref()
            .map(|windowing| windowing.display_api_handle.clone())
    }

    pub fn new_with_machine(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomManager>,
        machine_factories: MachineFactories<RS::GraphicsApi>,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) -> Self {
        let mut me = Self::new(environment.clone(), rom_manager.clone(), machine_factories);
        me.pending_machine_resources = Some((game_system, user_specified_roms));
        me.mode = RuntimeMode::Machine;

        me
    }

    pub fn insert_input(&mut self, id: GamepadId, input: Input, state: InputState) {
        self.current_key_states
            .entry(id)
            .or_default()
            .insert(input, state);

        // check for hotkeys

        let enviroment_guard = self.environment.read().unwrap();

        for (keys_to_press, action) in enviroment_guard.hotkeys.iter() {
            if keys_to_press.iter().all(|key| {
                self.current_key_states
                    .get(&id)
                    .and_then(|map| map.get(key))
                    .map(|state| state.as_digital(None))
                    .unwrap_or(false)
            }) {
                tracing::debug!("Hotkey pressed: {:?}", action);

                match action {
                    Hotkey::ToggleMenu => {
                        let maybe_machine_guard = self.maybe_machine.read().unwrap();

                        match self.mode {
                            RuntimeMode::Machine => {
                                self.mode = RuntimeMode::Gui;
                            }
                            RuntimeMode::Gui => {
                                if maybe_machine_guard.is_some() {
                                    self.mode = RuntimeMode::Machine;
                                }
                            }
                        }
                    }
                    Hotkey::FastForward => todo!(),
                    Hotkey::LoadSnapshot => todo!(),
                    Hotkey::SaveSnapshot => todo!(),
                }
            }
        }

        match &self.mode {
            RuntimeMode::Machine => {
                let maybe_machine_guard = self.maybe_machine.read().unwrap();
                let maybe_machine_guard = maybe_machine_guard.as_ref().unwrap();

                if let Some(virtual_id) = self.gamepad_mapping.get(&id) {
                    if let Some(virtual_gamepad) =
                        maybe_machine_guard.virtual_gamepads.get(virtual_id)
                    {
                        if let Some(transformed_input) = enviroment_guard
                            .gamepad_configs
                            .get(&maybe_machine_guard.game_system)
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
            RuntimeMode::Gui => {}
        }
    }

    pub fn redraw(&mut self, custom_gui_input: Option<RawInput>) {
        let windowing = self.windowing_context.as_mut().unwrap();

        if self.previous_window_size != windowing.display_api_handle.dimensions() {
            windowing.state.surface_resized();
            self.previous_window_size = windowing.display_api_handle.dimensions();
        }
        let mut maybe_machine_guard = self.maybe_machine.write().unwrap();

        match &mut self.mode {
            RuntimeMode::Machine => {
                let maybe_machine_guard = maybe_machine_guard.as_mut().unwrap();

                let start_period = Instant::now();

                let start_schedule_period = std::time::Instant::now();
                maybe_machine_guard.scheduler.run();
                let elapsed = start_schedule_period.elapsed();
                let scheduler_alloted_time =
                    self.previous_frame_time - self.previous_frame_render_time;

                match elapsed.cmp(&scheduler_alloted_time) {
                    std::cmp::Ordering::Less => {
                        maybe_machine_guard.scheduler.speed_up();
                    }
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Greater => {
                        maybe_machine_guard.scheduler.slow_down();
                    }
                }

                let start_frame_render_period = Instant::now();
                windowing.state.redraw(maybe_machine_guard);
                let frame_render_duration = start_frame_render_period.elapsed();
                let frame_duration = start_period.elapsed();

                self.previous_frame_time = frame_duration;
                self.previous_frame_render_time = frame_render_duration;
            }
            RuntimeMode::Gui => {
                let mut ui_output = None;

                let full_output = self.egui_context.clone().run(
                    custom_gui_input.unwrap_or_default(),
                    |context| {
                        ui_output = ui_output.take().or(self
                            .menu_state
                            .run_menu(context, maybe_machine_guard.as_ref()));
                    },
                );

                match ui_output {
                    None => {}
                    Some(UiOutput::Resume) => {
                        if maybe_machine_guard.is_some() {
                            self.mode = RuntimeMode::Machine;
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
                        drop(maybe_machine_guard);

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
