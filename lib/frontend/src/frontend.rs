use std::{
    collections::{HashMap, HashSet},
    num::Wrapping,
    sync::Arc,
    time::{Duration, Instant},
};

use multiemu_runtime::{
    graphics::GraphicsApi,
    input::{RealGamepad, RealGamepadId},
    machine::{Machine, graphics::GraphicsRequirements},
    persistence::SnapshotSlot,
    program::{ProgramManager, ProgramSpecification},
};
use nalgebra::Vector2;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rustc_hash::FxBuildHasher;

use crate::{
    EguiWindowingIntegration, GraphicsRuntime, Hotkey, MachineFactories, PlatformExt,
    WindowingHandle,
    backend::AudioRuntime,
    environment::Environment,
    gui::{GuiState, MenuOutput},
};

#[derive(Debug)]
struct PendingMachineResources {
    pub program_specification: ProgramSpecification,
}

#[derive(Debug)]
/// Windowing dependent types
pub struct WindowingContext<P: PlatformExt> {
    handle: <P::GraphicsRuntime as GraphicsRuntime<P>>::WindowingHandle,
    graphics_runtime: P::GraphicsRuntime,
}

/// Type alias for a possibly active machine

#[derive(Debug)]
struct GamepadData {
    gamepad: Arc<RealGamepad>,
    /// So hotkeys don't get triggered while continously holding them
    throttle_hotkey: bool,
}

#[derive(Debug)]
/// Frontend for the emulator
pub struct Frontend<P: PlatformExt> {
    // Current machine
    machine: Option<Arc<Machine>>,
    /// Gamepads connected
    gamepads: HashMap<RealGamepadId, GamepadData, FxBuildHasher>,
    /// The rom manager in use
    pub(crate) program_manager: Arc<ProgramManager>,
    /// Environment to read/modify
    pub environment: Environment,
    /// Our windowing and rendering context
    windowing_context: Option<WindowingContext<P>>,
    /// Bits and pieces of persistant menu state
    pub(crate) gui: GuiState<P>,
    /// The size the window was last time we checked
    previous_window_size: Vector2<u32>,
    /// Factories to construct a machine
    machine_factories: MachineFactories<P>,
    /// Data that the machine simulator needs when it's possible to build it
    pending_machine_resources: Option<PendingMachineResources>,
    /// The runtime for audio
    audio_runtime: P::AudioRuntime,
    /// The previous framerates we observed
    collected_frame_timings: ConstGenericRingBuffer<Duration, 5>,
    /// The timestamp previous frame
    previous_frame_timestamp: Instant,
    /// If we are in focus right now
    in_focus: bool,
    /// The current snapshot slot
    current_snapshot_slot: Wrapping<SnapshotSlot>,
    /// If the egui context needs to be reset because graphics was
    need_egui_reset: bool,
}

impl<P: PlatformExt> Frontend<P> {
    /// Create a new runtime
    pub fn new(
        environment: Environment,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<P>,
    ) -> Self {
        let machine = None;
        let gui = GuiState::new(&environment);
        let gamepads = HashMap::default();
        let audio_runtime = P::AudioRuntime::new();

        audio_runtime.play();

        Self {
            machine,
            gamepads,
            environment,
            program_manager,
            gui,
            windowing_context: None,
            previous_window_size: Vector2::zeros(),
            machine_factories,
            pending_machine_resources: None,
            audio_runtime,
            collected_frame_timings: ConstGenericRingBuffer::default(),
            previous_frame_timestamp: Instant::now(),
            in_focus: true,
            current_snapshot_slot: Wrapping(0),
            need_egui_reset: false,
        }
    }

    /// Create a new runtime a machine that needs to be built
    pub fn new_with_machine(
        environment: Environment,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<P>,
        program_specification: ProgramSpecification,
    ) -> Self {
        let mut me = Self::new(environment, program_manager, machine_factories);
        me.pending_machine_resources = Some(PendingMachineResources {
            program_specification,
        });
        me
    }

    /// Denote that some kind of focus change occured
    pub fn focus_change(&mut self, focused: bool) {
        self.collected_frame_timings.clear();
        self.previous_frame_timestamp = Instant::now();

        self.in_focus = focused;
    }

    pub fn machine(&self) -> Option<&Machine> {
        self.machine.as_deref()
    }

    /// Get access to the inner egui platform integration type
    pub fn get_windowing_integration(&mut self) -> Option<&mut P::EguiWindowingIntegration> {
        self.gui.get_windowing_integration()
    }

    /// Late initialization of the display API handle
    ///
    /// Comes up on desktop platforms since they send to give a window handle
    /// asynchronously
    pub fn set_windowing_handle(
        &mut self,
        windowing_handle: <P::GraphicsRuntime as GraphicsRuntime<P>>::WindowingHandle,
        egui_windowing_integration: P::EguiWindowingIntegration,
    ) {
        if self.machine.is_some() {
            tracing::error!("Machine was set up before the display handle was given!");
        }

        // Make sure the context is reinitialized so that textures are reinitialized
        self.gui
            .set_windowing_integration(egui_windowing_integration);

        if let Some(PendingMachineResources {
            program_specification,
        }) = self.pending_machine_resources.take()
        {
            // setup_runtime_for_new_machine will relock mode but its ok because no threads
            // should be running here

            self.setup_runtime_for_new_machine(Some(windowing_handle), program_specification);
        } else {
            let graphics_runtime = GraphicsRuntime::new(
                windowing_handle.clone(),
                <P::GraphicsApi as GraphicsApi>::Features::default(),
                <P::GraphicsApi as GraphicsApi>::Features::default(),
                &self.environment,
            )
            .unwrap();

            let windowing = WindowingContext {
                handle: windowing_handle,
                graphics_runtime,
            };
            self.windowing_context = Some(windowing);
            self.need_egui_reset = true;
        }
    }

    pub fn insert_gamepad(&mut self, id: RealGamepadId, real_gamepad: Arc<RealGamepad>) {
        tracing::info!(
            "Gamepad with id {} and name {} added",
            id,
            real_gamepad.metadata().name
        );

        self.gamepads.insert(
            id,
            GamepadData {
                gamepad: real_gamepad,
                throttle_hotkey: false,
            },
        );
    }

    pub fn remove_gamepad(&mut self, id: RealGamepadId) {
        self.gamepads.remove(&id);
    }

    pub fn get_gamepad(&mut self, id: RealGamepadId) -> Option<Arc<RealGamepad>> {
        self.gamepads.get(&id).map(|data| data.gamepad.clone())
    }

    /// Redraw for the runtime
    pub fn redraw(&mut self) {
        if !self.in_focus {
            return;
        }

        if self.need_egui_reset {
            self.gui.reset_context();
            self.need_egui_reset = false;
        }

        self.handle_virtual_and_hotkey_inputs();

        let windowing = self.windowing_context.as_mut().unwrap();

        let new_window_dimensions = windowing.handle.dimensions();
        if self.previous_window_size != new_window_dimensions {
            windowing.graphics_runtime.display_resized();
            self.previous_window_size = new_window_dimensions;
        }

        self.collected_frame_timings
            .enqueue(self.previous_frame_timestamp.elapsed());
        self.previous_frame_timestamp = Instant::now();

        if let Some(machine) = self.machine.as_mut()
            && !self.gui.active
        {
            // If the scheduler state is here, we must manually drive it
            let frame_timing = if self.collected_frame_timings.is_empty() {
                Duration::from_secs(1) / 60
            } else {
                self.collected_frame_timings.iter().sum::<Duration>()
                    / self.collected_frame_timings.len() as u32
            };

            machine.run_duration(frame_timing);
        }

        // TODO: Account for out of windowing systems input
        let input = self
            .gui
            .get_windowing_integration()
            .unwrap()
            .gather_platform_specific_inputs();
        let MenuOutput {
            egui_output,
            new_program,
        } = self.run_menu(input);

        let egui_context = self.gui.context();
        let windowing = self.windowing_context.as_mut().unwrap();

        windowing.graphics_runtime.redraw(
            egui_context,
            egui_output,
            self.machine.as_deref(),
            &self.environment,
        );

        if let Some(new_program) = new_program {
            self.setup_runtime_for_new_machine(None, new_program);
        }
    }

    fn setup_runtime_for_new_machine(
        &mut self,
        windowing_handle: Option<<P::GraphicsRuntime as GraphicsRuntime<P>>::WindowingHandle>,
        program_specification: ProgramSpecification,
    ) {
        let windowing_handle = {
            let old_windowing_context = self.windowing_context.take();
            windowing_handle.unwrap_or_else(|| old_windowing_context.map(|w| w.handle).unwrap())
        };

        self.machine = None;
        let machine_id = program_specification.id.machine;

        let machine_builder = Machine::build(
            Some(program_specification),
            self.program_manager.clone(),
            Some(self.environment.save_directory.clone()),
            Some(self.environment.snapshot_directory.clone()),
            self.audio_runtime.sample_rate(),
        );

        let machine_builder = self.machine_factories.construct_machine(machine_builder);

        let GraphicsRequirements {
            required_features,
            preferred_features,
        } = machine_builder.graphics_requirements();

        let graphics_runtime = P::GraphicsRuntime::new(
            windowing_handle.clone(),
            required_features,
            preferred_features,
            &self.environment,
        )
        .unwrap();

        let machine = machine_builder.build(graphics_runtime.component_initialization_data());

        let windowing = WindowingContext {
            handle: windowing_handle,
            graphics_runtime,
        };
        self.windowing_context = Some(windowing);

        // Make sure default mappings exist
        for (path, virtual_gamepad) in &machine.virtual_gamepads {
            for real_gamepad_id in self.gamepads.keys() {
                self.environment
                    .input
                    .gamepads
                    .entry((machine_id, path.clone()))
                    .or_default()
                    .0
                    .entry(*real_gamepad_id)
                    .or_insert_with(|| {
                        virtual_gamepad
                            .metadata()
                            .default_real2virtual_mappings
                            .clone()
                            .into_iter()
                            .collect()
                    });
            }
        }

        self.machine = Some(machine);
        self.need_egui_reset = true;
        self.gui.active = false;
    }

    fn handle_virtual_and_hotkey_inputs(&mut self) {
        // Check if any gamepad is mashing a hotkey
        for (real_gamepad_id, real_gamepad_data) in &mut self.gamepads {
            let mut inputs_relevant_to_hotkeys = HashSet::new();

            for (keys_to_press, action) in &self.environment.hotkeys {
                if keys_to_press
                    .iter()
                    .all(|key| real_gamepad_data.gamepad.get(*key).as_digital(None))
                {
                    // Record what keys participated in hotkeys this run
                    inputs_relevant_to_hotkeys.extend(keys_to_press.iter().copied());

                    // Make sure there are actually hotkeys, all the keys are pressed, and we are
                    // not throttling
                    if !keys_to_press.is_empty() && !real_gamepad_data.throttle_hotkey {
                        tracing::debug!(
                            "Hotkey pressed: {:?} with gamepad {}",
                            action,
                            real_gamepad_id
                        );

                        inputs_relevant_to_hotkeys.extend(keys_to_press.iter().copied());

                        match action {
                            Hotkey::ToggleMenu => {
                                if self.machine.is_some() {
                                    self.gui.active = !self.gui.active;
                                } else {
                                    self.gui.active = true;
                                }
                            }
                            Hotkey::FastForward => {}
                            Hotkey::LoadSnapshot => {}
                            Hotkey::StoreSnapshot => {}
                            Hotkey::IncrementSnapshotCounter => {
                                self.current_snapshot_slot += 1;
                            }
                            Hotkey::DecrementSnapshotCounter => {
                                self.current_snapshot_slot -= 1;
                            }
                            _ => {}
                        }

                        real_gamepad_data.throttle_hotkey = true;
                    }
                }
            }

            if inputs_relevant_to_hotkeys.is_empty() {
                real_gamepad_data.throttle_hotkey = false;
            }

            // Copy real inputs into virtual inputs

            if !self.gui.active
                && let Some(machine) = self.machine.as_ref()
            {
                let machine_id = machine
                    .program_specification
                    .as_ref()
                    .map(|program_specification| program_specification.id.machine)
                    .unwrap();

                for ((_, virtual_gamepad_path), real2virtual_mappings) in self
                    .environment
                    .input
                    .gamepads
                    .iter()
                    .filter(|((config_machine_id, _), _)| config_machine_id == &machine_id)
                {
                    if let Some(virtual_gamepad) =
                        machine.virtual_gamepads.get(virtual_gamepad_path)
                        && let Some(real2virtual_mapping) =
                            real2virtual_mappings.0.get(real_gamepad_id)
                    {
                        // Scan real gamepad for its inputs and transform them into the virtual
                        // gamepad
                        //
                        // Do not check inputs that were part of a hotkey

                        for real_present_input in real_gamepad_data
                            .gamepad
                            .metadata()
                            .present_inputs
                            .iter()
                            .filter(|input| !inputs_relevant_to_hotkeys.contains(input))
                        {
                            if let Some(transformed_input) =
                                real2virtual_mapping.get(real_present_input)
                            {
                                let state = real_gamepad_data.gamepad.get(*real_present_input);

                                tracing::trace!(
                                    "Transformed input: {:?} -> {:?} to state {:?}",
                                    real_present_input,
                                    transformed_input,
                                    state
                                );

                                virtual_gamepad.set(*transformed_input, state);
                            }
                        }
                    }
                }
            }
        }
    }
}
