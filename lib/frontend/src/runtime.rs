use crate::{
    EguiPlatformIntegration,
    backend::{AudioContext, DisplayApiHandle, GraphicsRuntime},
    gui::menu::{MenuState, UiOutput},
    machine_factories::MachineFactories,
    platform::PlatformExt,
};
use egui::{FontFamily, TextStyle, TextWrapMode};
use multiemu_config::Environment;
use multiemu_graphics::GraphicsApi;
use multiemu_input::{
    GamepadId, Input, InputState,
    hotkey::{DEFAULT_HOTKEYS, Hotkey},
};
use multiemu_rom::{GameSystem, ROM_INFORMATION_TABLE, RomId, RomManager};
use multiemu_runtime::{
    Machine, graphics::GraphicsRequirements, input::VirtualGamepadId, platform::Platform,
};
use nalgebra::Vector2;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use std::{
    collections::HashMap,
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
struct PendingMachineResources {
    pub game_system: GameSystem,
    pub user_specified_roms: Vec<RomId>,
}

#[derive(Debug)]
pub struct WindowingContext<P: PlatformExt> {
    display_api_handle: <P::GraphicsRuntime as GraphicsRuntime<P>>::DisplayApiHandle,
    graphics_runtime: P::GraphicsRuntime,
    egui_platform_integration: P::EguiPlatformIntegration,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Mode {
    Gui,
    Machine,
}

pub type MaybeMachine<P> = RwLock<Option<Machine<P>>>;

#[derive(Debug)]
pub struct StoredMachine<P: Platform> {
    pub maybe_machine: Arc<MaybeMachine<P>>,
    pub system: Option<GameSystem>,
}

#[derive(Debug)]
pub struct FrontendRuntime<P: PlatformExt> {
    /// Main swappable machine
    stored_machine: StoredMachine<P>,
    /// What mode the runtime is in right now
    mode: Mode,
    /// Gamepad to emulated gamepad mappings
    gamepad_mapping: HashMap<GamepadId, VirtualGamepadId>,
    /// The rom manager in use
    rom_manager: Arc<RomManager>,
    /// Environment to read/modify
    environment: Arc<RwLock<Environment>>,
    /// Egui context
    egui_context: egui::Context,
    /// Our windowing and rendering context
    windowing_context: Option<WindowingContext<P>>,
    /// Bits and pieces of persistant menu state
    menu_state: MenuState,
    /// The size the window was last time we checked
    previous_window_size: Vector2<u32>,
    /// What the current state of various keys are
    current_key_states: HashMap<GamepadId, HashMap<Input, InputState>>,
    /// Factories to construct a machine
    machine_factories: MachineFactories<P>,
    /// Data that the machine simulator needs when it's possible to build it
    pending_machine_resources: Option<PendingMachineResources>,
    /// The runtime for audio
    audio_runtime: P::AudioRuntime,
    /// Way to execute on the main thread
    main_thread_executor: Arc<P::MainThreadExecutor>,
    /// The previous framerates we observed
    collected_frame_rates: ConstGenericRingBuffer<Duration, 5>,
    /// The timestamp previous frame
    previous_frame_timestamp: Instant,
    /// If we are in focus rn
    in_focus: bool,
}

impl<P: PlatformExt> FrontendRuntime<P> {
    pub fn focus_change(&mut self, focused: bool) {
        self.collected_frame_rates.clear();
        self.previous_frame_timestamp = Instant::now();

        self.in_focus = focused;
    }

    pub fn maybe_machine(&self) -> Arc<MaybeMachine<P>> {
        self.stored_machine.maybe_machine.clone()
    }

    pub fn egui_platform_integration(&mut self) -> &mut P::EguiPlatformIntegration {
        &mut self
            .windowing_context
            .as_mut()
            .unwrap()
            .egui_platform_integration
    }

    /// Late initialization of the display API handle
    ///
    /// Comes up on desktop platforms since they send to give a window handle asynchronously
    pub fn set_display_api_handle(
        &mut self,
        display_api_handle: <P::GraphicsRuntime as GraphicsRuntime<P>>::DisplayApiHandle,
        mut egui_platform_integration: P::EguiPlatformIntegration,
    ) {
        match self.mode {
            Mode::Machine => {
                unreachable!("Cannot reinit machine!")
            }
            Mode::Gui => {
                if let Some(PendingMachineResources {
                    game_system,
                    user_specified_roms,
                }) = self.pending_machine_resources.take()
                {
                    // setup_runtime_for_new_machine will relock mode but its ok because no threads should be running here

                    self.setup_runtime_for_new_machine(
                        display_api_handle,
                        game_system,
                        user_specified_roms,
                        egui_platform_integration,
                    );
                } else {
                    self.egui_context = egui::Context::default();
                    setup_theme(&self.egui_context);
                    egui_platform_integration.set_egui_context(&self.egui_context);

                    let graphics_runtime = GraphicsRuntime::new(
                        display_api_handle.clone(),
                        <P::GraphicsApi as GraphicsApi>::Features::default(),
                        <P::GraphicsApi as GraphicsApi>::Features::default(),
                        self.environment.clone(),
                    )
                    .unwrap();

                    let windowing = WindowingContext {
                        display_api_handle,
                        graphics_runtime,
                        egui_platform_integration,
                    };
                    self.windowing_context = Some(windowing);
                }
            }
        }
    }

    pub fn insert_input(&mut self, id: GamepadId, input: Input, state: InputState) {
        self.current_key_states
            .entry(id)
            .or_default()
            .insert(input, state);

        // check for hotkeys

        let maybe_machine_guard = self.stored_machine.maybe_machine.read().unwrap();
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
                    Hotkey::ToggleMenu => match self.mode {
                        Mode::Machine => {
                            self.mode = Mode::Gui;
                        }
                        Mode::Gui => {
                            if maybe_machine_guard.is_some() {
                                self.mode = Mode::Machine;
                            }
                        }
                    },
                    Hotkey::FastForward => todo!(),
                    Hotkey::LoadSnapshot => todo!(),
                    Hotkey::SaveSnapshot => todo!(),
                }
            }
        }

        match self.mode {
            Mode::Machine => {
                let machine = maybe_machine_guard.as_ref().unwrap();

                if let Some(virtual_id) = self.gamepad_mapping.get(&id) {
                    if let Some(virtual_gamepad) = machine.virtual_gamepads.get(virtual_id) {
                        if let Some(transformed_input) = enviroment_guard
                            .gamepad_configs
                            .get(&self.stored_machine.system.unwrap())
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
            Mode::Gui => {}
        }
    }

    pub fn redraw(&mut self) {
        if !self.in_focus {
            return;
        }

        let windowing = self.windowing_context.as_mut().unwrap();

        let new_window_dimensions = windowing.display_api_handle.dimensions();
        if self.previous_window_size != new_window_dimensions {
            windowing.graphics_runtime.display_resized();
            self.previous_window_size = new_window_dimensions;
        }
        let maybe_machine_guard = self.stored_machine.maybe_machine.read().unwrap();

        self.collected_frame_rates
            .push(Instant::now() - self.previous_frame_timestamp);
        self.previous_frame_timestamp = Instant::now();

        match self.mode {
            Mode::Machine => {
                let frame_timing: Duration = if self.collected_frame_rates.is_empty() {
                    Duration::from_secs(1) / 60
                } else {
                    self.collected_frame_rates.iter().sum::<Duration>()
                        / self.collected_frame_rates.len() as u32
                };

                let maybe_machine_guard = self.stored_machine.maybe_machine.read().unwrap();
                let maybe_machine = maybe_machine_guard.as_ref().unwrap();

                let render_frame_start_timestamp = Instant::now();
                windowing.graphics_runtime.redraw(maybe_machine);
                let render_frame_time_taken = Instant::now() - render_frame_start_timestamp;

                maybe_machine.scheduler.lock().unwrap().run(
                    frame_timing
                        .checked_sub(render_frame_time_taken)
                        .unwrap_or(frame_timing),
                );
            }
            Mode::Gui => {
                let mut ui_output = None;

                let full_output = self.egui_context.clone().run(
                    windowing
                        .egui_platform_integration
                        .gather_platform_specific_inputs(),
                    |context| {
                        ui_output = ui_output.take().or(self.menu_state.run_menu(context));
                    },
                );

                match ui_output {
                    None => {}
                    Some(UiOutput::Resume) => {
                        let is_machine_active = maybe_machine_guard.is_some();

                        if is_machine_active && self.mode == Mode::Gui {
                            self.mode = Mode::Machine;
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
                            graphics_runtime,
                            egui_platform_integration,
                        } = self.windowing_context.take().unwrap();

                        // Drop the graphics context less we get a terrible error
                        drop(graphics_runtime);
                        drop(maybe_machine_guard);

                        self.setup_runtime_for_new_machine(
                            display_api_handle,
                            rom_info.system,
                            vec![rom_id],
                            egui_platform_integration,
                        );

                        return;
                    }
                }

                self.windowing_context
                    .as_mut()
                    .unwrap()
                    .graphics_runtime
                    .redraw_menu(&self.egui_context, full_output);
            }
        }
    }
}

impl<P: PlatformExt> FrontendRuntime<P> {
    pub fn new(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomManager>,
        machine_factories: MachineFactories<P>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> Self {
        let maybe_machine = Arc::new(MaybeMachine::default());
        let egui_context = egui::Context::default();
        setup_theme(&egui_context);
        let menu_state = MenuState::new(environment.clone(), rom_manager.clone());
        let gamepad_mapping = HashMap::new();
        let audio_runtime = P::AudioRuntime::new(maybe_machine.clone());

        // Make sure we have some hotkeys at least
        let mut environment_guard = environment.write().unwrap();
        if environment_guard.hotkeys.is_empty() {
            environment_guard.hotkeys = DEFAULT_HOTKEYS.clone();
        }
        drop(environment_guard);

        audio_runtime.play();

        Self {
            mode: Mode::Gui,
            stored_machine: StoredMachine {
                maybe_machine,
                system: None,
            },
            gamepad_mapping,
            environment,
            rom_manager,
            egui_context,
            menu_state,
            windowing_context: None,
            previous_window_size: Vector2::zeros(),
            current_key_states: HashMap::new(),
            machine_factories,
            pending_machine_resources: None,
            audio_runtime,
            main_thread_executor,
            collected_frame_rates: ConstGenericRingBuffer::default(),
            previous_frame_timestamp: Instant::now(),
            in_focus: true,
        }
    }

    pub fn new_with_machine(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomManager>,
        machine_factories: MachineFactories<P>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) -> Self {
        let mut me = Self::new(
            environment.clone(),
            rom_manager.clone(),
            machine_factories,
            main_thread_executor,
        );
        me.pending_machine_resources = Some(PendingMachineResources {
            game_system,
            user_specified_roms,
        });
        me
    }

    pub(super) fn setup_runtime_for_new_machine(
        &mut self,
        display_api_handle: <P::GraphicsRuntime as GraphicsRuntime<P>>::DisplayApiHandle,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
        mut egui_platform_integration: P::EguiPlatformIntegration,
    ) {
        let mut maybe_machine_guard = self.stored_machine.maybe_machine.write().unwrap();

        // Drop old machine otherwise it will segfault when we try to use the new video context
        // I dunno how we could prevent this unsafety but beware future programmers

        maybe_machine_guard.take();
        self.stored_machine.system = None;

        let machine_builder = self.machine_factories.construct_machine(
            game_system,
            user_specified_roms,
            self.rom_manager.clone(),
            self.audio_runtime.sample_rate(),
            self.main_thread_executor.clone(),
        );

        self.egui_context = egui::Context::default();
        setup_theme(&self.egui_context);

        egui_platform_integration.set_egui_context(&self.egui_context);

        let GraphicsRequirements {
            required_features,
            preferred_features,
        } = machine_builder.graphics_requirements();

        let graphics_runtime = P::GraphicsRuntime::new(
            display_api_handle.clone(),
            required_features,
            preferred_features,
            self.environment.clone(),
        )
        .unwrap();

        let machine = machine_builder.build(graphics_runtime.component_initialization_data());

        let windowing = WindowingContext {
            display_api_handle,
            graphics_runtime,
            egui_platform_integration,
        };
        self.windowing_context = Some(windowing);

        // HACK: Just map the keyboard since we have nothing else set up
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
                .entry(game_system)
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

        *maybe_machine_guard = Some(machine);
        self.stored_machine.system = Some(game_system);
        self.mode = Mode::Machine;
    }
}
