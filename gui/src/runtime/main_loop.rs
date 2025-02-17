use std::any::TypeId;
use std::fs::File;
use std::sync::{Arc, RwLock};

use crate::build_machine::build_machine;
use crate::gui::menu::UiOutput;
use crate::{gui::menu::MenuState, rendering_backend::RenderingBackendState};
use crossbeam::channel::{Receiver, TryRecvError};
use egui::ViewportId;
use multiemu_config::Environment;
use multiemu_input::{Input, InputState};
use multiemu_machine::builder::display::BackendSpecificData;
use multiemu_machine::display::{ContextExtensionSpecification, RenderBackend};
use multiemu_machine::Machine;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::{LoadedRomLocation, RomManager, ROM_INFORMATION_TABLE};
use multiemu_rom::system::GameSystem;
use winit::window::Window;

pub enum Message {
    Input {
        input: Input,
        state: InputState,
    },
    RunMachine {
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    },
}

enum RuntimeMode {
    Idle,
    Pending {
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    },
    Running {
        machine: Machine,
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
    mode: RuntimeMode,
    egui_winit: egui_winit::State,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
}

impl<
        R: RenderBackend,
        RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>,
    > MainLoop<R, RS>
{
    pub fn new(
        message_channel: Receiver<Message>,
        display_api_handle: RS::DisplayApiHandle,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) -> Self {
        let menu_state = MenuState::new(environment.clone());

        Self {
            message_channel,
            rendering_backend: RS::new(
                display_api_handle.clone(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
                <R as RenderBackend>::ContextExtensionSpecification::default(),
                environment.clone(),
            )
            .unwrap(),
            egui_winit: egui_winit::State::new(
                menu_state.egui_context.clone(),
                ViewportId::ROOT,
                &display_api_handle,
                None,
                None,
                None,
            ),
            menu_state,
            menu_active: false,
            mode: RuntimeMode::Idle,
            rom_manager,
            environment,
            display_api_handle,
        }
    }

    pub fn run(&mut self) {
        loop {
            loop {
                match self.message_channel.try_recv() {
                    Ok(message) => match message {
                        Message::Input { input, state } => todo!(),
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

            if matches!(self.mode, RuntimeMode::Idle) {
                self.menu_active = true;
            }

            if self.menu_active {
                // We put the ui output like this so multipassing egui gui building works
                let mut ui_output = None;
                let full_output = self.menu_state.egui_context.clone().run(
                    self.egui_winit.take_egui_input(&self.display_api_handle),
                    |context| {
                        ui_output = ui_output
                            .take()
                            .or(self.menu_state.run_menu(context, &self.rom_manager));
                    },
                );

                match ui_output {
                    None => {}
                    Some(UiOutput::OpenGame { path }) => {
                        tracing::info!("Opening rom at {}", path.display());

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

                            /*
                            // Make sure the system being run has a default mapping
                            let mut environment_guard = self.environment.write().unwrap();
                            */

                            self.mode = RuntimeMode::Pending {
                                game_system,
                                user_specified_roms: vec![rom_id],
                            }
                        } else {
                            tracing::error!("Could not identify rom at {}", path.display());
                        }
                    }
                }

                self.rendering_backend
                    .redraw_menu(&self.menu_state.egui_context, full_output);
            } else {
                self.rendering_backend.redraw();
            }

            if let RuntimeMode::Pending {
                game_system,
                user_specified_roms,
            } = std::mem::replace(&mut self.mode, RuntimeMode::Idle)
            {
                let machine_builder = build_machine(
                    game_system,
                    user_specified_roms,
                    self.rom_manager.clone(),
                    self.environment.clone(),
                );

                let mut preferred_extensions = <<RS as RenderingBackendState>::RenderBackend as RenderBackend>::ContextExtensionSpecification::default();
                let mut required_extensions = <<RS as RenderingBackendState>::RenderBackend as RenderBackend>::ContextExtensionSpecification::default();

                for (_component_id, component) in machine_builder.component_metadata.iter() {
                    if let Some(display) = &component.display {
                        let backend_specific_data = display
                            .backend_specific_data
                            .get(&TypeId::of::<<RS as RenderingBackendState>::RenderBackend>())
                            .map(|data| {
                                let data = data
                                    .downcast_ref::<BackendSpecificData<
                                        <RS as RenderingBackendState>::RenderBackend,
                                    >>()
                                    .unwrap();

                                data
                            })
                            .expect("Could not find display backend data for component");

                        preferred_extensions = preferred_extensions
                            .combine(backend_specific_data.preferred_extensions.clone());
                        required_extensions = required_extensions
                            .combine(backend_specific_data.required_extensions.clone());
                    }
                }

                self.mode = RuntimeMode::Running {
                    machine: machine_builder
                        .build::<R>(self.rendering_backend.component_initialization_data()),
                };
            }
        }
    }
}
