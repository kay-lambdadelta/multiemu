use crate::{
    DisplayApiHandle, EguiPlatformIntegration, GraphicsRuntime, MachineFactories, PlatformExt,
    backend::AudioRuntime,
    gui::{MenuState, UiOutput, setup_theme},
};
use multiemu_runtime::{
    component::ResourcePath,
    environment::Environment,
    graphics::GraphicsApi,
    input::{
        GamepadId, Input, InputState,
        hotkey::{DEFAULT_HOTKEYS, Hotkey},
    },
    machine::{Machine, graphics::GraphicsRequirements},
    persistence::SnapshotSlot,
    program::{ProgramManager, ProgramSpecification},
};
use nalgebra::Vector2;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use std::{
    collections::HashMap,
    num::Wrapping,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

#[derive(Debug)]
struct PendingMachineResources {
    pub program_specification: ProgramSpecification,
}

#[derive(Debug)]
/// Windowing dependent types
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

/// A machine that may be there
pub type MaybeMachine<P> = RwLock<Option<Machine<P>>>;

#[derive(Debug)]
/// Frontend for the emulator
pub struct FrontendRuntime<P: PlatformExt> {
    /// Main swappable machine
    maybe_machine: Arc<MaybeMachine<P>>,
    /// What mode the runtime is in right now
    mode: Mode,
    /// Gamepad to emulated gamepad mappings
    gamepad_mapping: HashMap<GamepadId, ResourcePath>,
    /// The rom manager in use
    program_manager: Arc<ProgramManager>,
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
    /// The previous framerates we observed
    collected_frame_rates: ConstGenericRingBuffer<Duration, 5>,
    /// The timestamp previous frame
    previous_frame_timestamp: Instant,
    /// If we are in focus rn
    in_focus: bool,
    /// The current snapshot slot
    current_snapshot_slot: Wrapping<SnapshotSlot>,
}

impl<P: PlatformExt> FrontendRuntime<P> {
    /// Denote that some kind of focus change occured
    pub fn focus_change(&mut self, focused: bool) {
        self.collected_frame_rates.clear();
        self.previous_frame_timestamp = Instant::now();

        self.in_focus = focused;
    }

    /// Get the [`MaybeMachine`]
    pub fn maybe_machine(&self) -> Arc<MaybeMachine<P>> {
        self.maybe_machine.clone()
    }

    /// Get access to the inner egui platform integration type
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
                    program_specification,
                }) = self.pending_machine_resources.take()
                {
                    // setup_runtime_for_new_machine will relock mode but its ok because no threads should be running here

                    self.setup_runtime_for_new_machine(
                        display_api_handle,
                        program_specification,
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

    /// Inserts a input into the frontend
    pub fn insert_input(&mut self, id: GamepadId, input: Input, state: InputState) {
        self.current_key_states
            .entry(id)
            .or_default()
            .insert(input, state);

        // check for hotkeys

        let maybe_machine_guard = self.maybe_machine.read().unwrap();
        let enviroment_guard = self.environment.read().unwrap();

        for (keys_to_press, action) in &enviroment_guard.hotkeys {
            if keys_to_press.iter().all(|key| {
                self.current_key_states
                    .get(&id)
                    .and_then(|map| map.get(key))
                    .is_some_and(|state| state.as_digital(None))
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
                    Hotkey::LoadSnapshot if self.mode == Mode::Machine => {}
                    Hotkey::StoreSnapshot if self.mode == Mode::Machine => {}
                    Hotkey::IncrementSnapshotCounter => {
                        self.current_snapshot_slot += 1;
                    }
                    Hotkey::DecrementSnapshotCounter => {
                        self.current_snapshot_slot -= 1;
                    }
                    _ => {}
                }
            }
        }

        match self.mode {
            Mode::Machine => {
                let machine = maybe_machine_guard.as_ref().unwrap();
                let machine_id = machine
                    .program_specification
                    .as_ref()
                    .map(|ps| ps.id.machine)
                    .unwrap();

                if let Some(path) = self.gamepad_mapping.get(&id)
                    && let Some(virtual_gamepad) = machine.virtual_gamepads.get(path)
                    && let Some(transformed_input) = enviroment_guard
                        .gamepad_configs
                        .get(&machine_id)
                        .and_then(|gamepad_types| gamepad_types.get(path))
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
            Mode::Gui => {
                // TODO: Investigate if egui needs insertion here. Currently we are relying on the platform integration alone to insert inputs, but this may not always be intelligent on nonwinit platforms
            }
        }
    }

    /// Redraw for the runtime
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

        self.collected_frame_rates
            .enqueue(self.previous_frame_timestamp.elapsed());
        self.previous_frame_timestamp = Instant::now();

        match self.mode {
            Mode::Machine => {
                let mut maybe_machine_guard = self.maybe_machine.write().unwrap();
                let maybe_machine = maybe_machine_guard.as_mut().unwrap();

                // If the scheduler state is here, we must manually drive it
                if maybe_machine.scheduler.is_some() {
                    let frame_timing: Duration = if self.collected_frame_rates.is_empty() {
                        Duration::from_secs(1) / 60
                    } else {
                        self.collected_frame_rates.iter().sum::<Duration>()
                            / self.collected_frame_rates.len() as u32
                    };

                    let render_frame_start_timestamp = Instant::now();
                    windowing.graphics_runtime.redraw(maybe_machine);
                    let render_frame_time_taken = render_frame_start_timestamp.elapsed();

                    maybe_machine.scheduler.as_mut().unwrap().run(
                        frame_timing
                            .checked_sub(render_frame_time_taken)
                            .unwrap_or(frame_timing),
                    );
                } else {
                    windowing.graphics_runtime.redraw(maybe_machine);
                }
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
                let maybe_machine_guard = self.maybe_machine.read().unwrap();

                match ui_output {
                    None => {}
                    Some(UiOutput::Resume) => {
                        let is_machine_active = maybe_machine_guard.is_some();

                        if is_machine_active && self.mode == Mode::Gui {
                            self.mode = Mode::Machine;
                        }
                    }
                    Some(UiOutput::OpenProgram {
                        program_specification,
                    }) => {
                        let WindowingContext {
                            display_api_handle,
                            graphics_runtime,
                            egui_platform_integration,
                        } = self.windowing_context.take().unwrap();

                        // NOTE: We drop the graphics runtime here because many graphics runtimes cannot handle having two open at once
                        // Graphics runtimes should guard against unsafety for this if this is required for them
                        drop(graphics_runtime);
                        drop(maybe_machine_guard);

                        self.setup_runtime_for_new_machine(
                            display_api_handle,
                            program_specification,
                            egui_platform_integration,
                        );

                        return;
                    }
                    Some(UiOutput::Reset) => {}
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
    /// Create a new runtime
    pub fn new(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<P>,
    ) -> Self {
        let maybe_machine = Arc::new(MaybeMachine::default());
        let egui_context = egui::Context::default();
        setup_theme(&egui_context);
        let menu_state = MenuState::new(environment.clone(), program_manager.clone());
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
            maybe_machine,
            gamepad_mapping,
            environment,
            program_manager,
            egui_context,
            menu_state,
            windowing_context: None,
            previous_window_size: Vector2::zeros(),
            current_key_states: HashMap::new(),
            machine_factories,
            pending_machine_resources: None,
            audio_runtime,
            collected_frame_rates: ConstGenericRingBuffer::default(),
            previous_frame_timestamp: Instant::now(),
            in_focus: true,
            current_snapshot_slot: Wrapping(0),
        }
    }

    /// Create a new runtime a machine that needs to be built
    pub fn new_with_machine(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<P>,
        program_specification: ProgramSpecification,
    ) -> Self {
        let mut me = Self::new(
            environment.clone(),
            program_manager.clone(),
            machine_factories,
        );
        me.pending_machine_resources = Some(PendingMachineResources {
            program_specification,
        });
        me
    }

    pub(super) fn setup_runtime_for_new_machine(
        &mut self,
        display_api_handle: <P::GraphicsRuntime as GraphicsRuntime<P>>::DisplayApiHandle,
        program_specification: ProgramSpecification,
        mut egui_platform_integration: P::EguiPlatformIntegration,
    ) {
        let mut maybe_machine_guard = self.maybe_machine.write().unwrap();

        self.windowing_context.take();
        maybe_machine_guard.take();
        let machine_id = program_specification.id.machine;
        let environment_guard = self.environment.read().unwrap();

        let machine_builder = Machine::build(
            Some(program_specification),
            self.program_manager.clone(),
            Some(environment_guard.save_directory.clone()),
            Some(environment_guard.snapshot_directory.clone()),
            self.audio_runtime.sample_rate(),
        );

        let machine_builder = self.machine_factories.construct_machine(machine_builder);

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

        drop(environment_guard);
        let mut environment_guard = self.environment.write().unwrap();

        // HACK: Just map the keyboard since we have nothing else set up
        if let Some((virtual_gamepad, _)) = machine.virtual_gamepads.iter().next() {
            self.gamepad_mapping
                .insert(GamepadId::PLATFORM_RESERVED, virtual_gamepad.clone());
        }

        // Try to give the environment default bindings

        // Make sure default mappings exist
        for (path, virtual_gamepad) in &machine.virtual_gamepads {
            environment_guard
                .gamepad_configs
                .entry(machine_id)
                .or_default()
                .entry(path.clone())
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
        self.mode = Mode::Machine;
    }
}
