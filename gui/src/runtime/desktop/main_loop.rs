use crate::build_machine::build_machine;
use crate::gui::menu::UiOutput;
use crate::runtime::desktop::keyboard::KEYBOARD_ID;
use crate::timing_tracker::TimingTracker;
use crate::{gui::menu::MenuState, rendering_backend::RenderingBackendState};
use crossbeam::channel::{Receiver, TryRecvError};
use multiemu_config::Environment;
use multiemu_input::{GamepadId, Input, InputState};
use multiemu_machine::Machine;
use multiemu_machine::builder::display::BackendSpecificData;
use multiemu_machine::display::{ContextExtensionSpecification, RenderBackend};
use multiemu_machine::input::VirtualGamepadId;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::{LoadedRomLocation, ROM_INFORMATION_TABLE, RomManager};
use multiemu_rom::system::GameSystem;
use nalgebra::Vector2;
use std::any::TypeId;
use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex, RwLock};
use winit::window::Window;

pub enum Message {
    Input {
        id: GamepadId,
        input: Input,
        state: InputState,
    },
    RunMachine {
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    },
}

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

pub struct MainLoop<
    R: RenderBackend,
    RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>,
> {
    message_channel: Receiver<Message>,
    display_api_handle: RS::DisplayApiHandle,
    rendering_backend: RS,
    menu_state: MenuState,
    menu_active: bool,
    mode: RuntimeMode<R>,
    egui_winit: Arc<Mutex<egui_winit::State>>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    previously_seen_window_size: Vector2<u16>,
    timing_tracker: TimingTracker,
    gamepad_mapping: HashMap<GamepadId, VirtualGamepadId>,
}

impl<R: RenderBackend, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>>
    MainLoop<R, RS>
{
    pub fn new(
        message_channel: Receiver<Message>,
        display_api_handle: RS::DisplayApiHandle,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        egui_context: egui::Context,
        egui_winit: Arc<Mutex<egui_winit::State>>,
    ) -> Self {
        let menu_state = MenuState::new(egui_context, environment.clone());

        Self {
            message_channel,
            rendering_backend: RS::new(
                display_api_handle.clone(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
                environment.clone(),
            )
            .unwrap(),
            egui_winit,
            previously_seen_window_size: {
                let window_size = display_api_handle.inner_size();
                Vector2::new(window_size.width, window_size.height).cast::<u16>()
            },
            menu_state,
            menu_active: false,
            mode: RuntimeMode::Idle,
            rom_manager,
            environment,
            display_api_handle,
            timing_tracker: TimingTracker::default(),
            gamepad_mapping: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            loop {
                match self.message_channel.try_recv() {
                    Ok(message) => match message {
                        Message::Input { id, input, state } => {
                            tracing::trace!("Input received: {:?}: {:?} {:?}", id, input, state);

                            if let Some(virtual_id) = self.gamepad_mapping.get(&id) {
                                let environment_guard = self.environment.read().unwrap();

                                if let RuntimeMode::Running { machine } = &self.mode {
                                    if let Some(virtual_gamepad) =
                                        machine.virtual_gamepads().get(virtual_id)
                                    {
                                        if let Some(transformed_input) = environment_guard
                                            .gamepad_configs
                                            .get(&machine.game_system())
                                            .and_then(|gamepad_types| {
                                                gamepad_types.get(&virtual_gamepad.name())
                                            })
                                            .and_then(|gamepad_transformer| {
                                                gamepad_transformer.get(&input)
                                            })
                                        {
                                            virtual_gamepad.set(*transformed_input, state);
                                        }
                                    }
                                }
                            }
                        }
                        Message::RunMachine {
                            game_system,
                            user_specified_roms,
                        } => {
                            self.mode = RuntimeMode::Pending {
                                game_system,
                                user_specified_roms,
                            };
                        }
                    },
                    Err(TryRecvError::Empty) => break,
                    _ => {
                        tracing::error!("Underlying runtime shut down uncleanly");
                        return;
                    }
                }
            }

            // Detect a window resize
            let window_size = self.display_api_handle.inner_size();
            let window_size = Vector2::new(window_size.width, window_size.height).cast::<u16>();

            if window_size != self.previously_seen_window_size {
                self.previously_seen_window_size = window_size;
                self.rendering_backend.surface_resized();
            }

            if !matches!(self.mode, RuntimeMode::Running { .. }) {
                self.menu_active = true;
            }

            if self.menu_active {
                let mut egui_winit_guard = self.egui_winit.lock().unwrap();

                // We put the ui output like this so multipassing egui gui building works
                let mut ui_output = None;
                let full_output = self.menu_state.egui_context.clone().run(
                    egui_winit_guard.take_egui_input(&self.display_api_handle),
                    |context| {
                        ui_output = ui_output
                            .take()
                            .or(self.menu_state.run_menu(context, &self.rom_manager));
                    },
                );
                drop(egui_winit_guard);

                match ui_output {
                    None => {}
                    Some(UiOutput::Resume) => {
                        self.menu_active = false;
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

                            self.mode = RuntimeMode::Pending {
                                game_system,
                                user_specified_roms: vec![rom_id],
                            }
                        } else {
                            tracing::error!("Could not identify ROM at {}", path.display());
                        }
                    }
                }

                self.rendering_backend
                    .redraw_menu(&self.menu_state.egui_context, full_output);
            } else {
                let RuntimeMode::Running { machine } = &mut self.mode else {
                    unreachable!()
                };

                self.timing_tracker.machine_main_cycle_starting();
                machine.scheduler.run();
                self.rendering_backend.redraw(machine);
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

            if matches!(self.mode, RuntimeMode::Pending { .. }) {
                let RuntimeMode::Pending {
                    game_system,
                    user_specified_roms,
                } = std::mem::replace(&mut self.mode, RuntimeMode::Idle)
                else {
                    unreachable!()
                };

                tracing::info!("Starting up machine for {}", game_system);

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
                        required_extensions = required_extensions
                            .combine(backend_specific_data.required_extensions.clone());
                    }
                }

                let machine = machine_builder
                    .build::<R>(self.rendering_backend.component_initialization_data());

                // HACK: Map the keyboard to the first gamepad

                if let Some(virtual_gamepad_id) = machine.virtual_gamepads().keys().next().copied()
                {
                    self.gamepad_mapping.insert(KEYBOARD_ID, virtual_gamepad_id);
                }

                self.mode = RuntimeMode::Running { machine };

                self.menu_active = false;
            }
        }
    }
}
