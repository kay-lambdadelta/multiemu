use crate::gui::menu::MenuState;
use crate::runtime::Runtime;
use crate::{rendering_backend::RenderingBackendState, timing_tracker::TimingTracker};
use audio::CpalAudio;
use egui::{FontFamily, TextStyle, TextWrapMode};
use gamepad::gamepad_task;
use multiemu_config::Environment;
use multiemu_input::{GamepadId, Input, InputState};
use multiemu_machine::Machine;
use multiemu_machine::display::RenderBackend;
use multiemu_machine::input::VirtualGamepadId;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::GameSystem;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use windowing::WindowingContext;
use winit::{event_loop::EventLoop, window::Window};

mod audio;
mod gamepad;
mod keyboard;
pub mod renderer;
mod windowing;

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
    egui_context: egui::Context,
    menu_state: MenuState,
    audio: CpalAudio,
    timing_tracker: TimingTracker,
    gamepad_mapping: HashMap<GamepadId, VirtualGamepadId>,
    mode: RuntimeMode<RS::RenderBackend>,
    gui_active: bool,
}

impl<R: RenderBackend, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>>
    Runtime<R> for PlatformRuntime<RS>
{
    fn launch_gui(rom_manager: Arc<RomManager>, environment: Arc<RwLock<Environment>>) {
        let egui_context = egui::Context::default();
        let menu_state = MenuState::new(environment.clone(), rom_manager.clone());

        setup_theme(&egui_context);

        let mut me = Self {
            windowing: None,
            rom_manager,
            environment,
            egui_context,
            menu_state,
            audio: CpalAudio::default(),
            timing_tracker: TimingTracker::default(),
            gamepad_mapping: HashMap::new(),
            mode: RuntimeMode::Idle,
            gui_active: true,
        };

        let event_loop = EventLoop::with_user_event().build().unwrap();
        {
            let event_loop_proxy = event_loop.create_proxy();

            std::thread::Builder::new()
                .name("gamepad".to_string())
                .spawn(move || {
                    tracing::debug!("Starting up gamepad thread");

                    gamepad_task(event_loop_proxy);

                    tracing::debug!("Shutting down gamepad thread");
                })
                .unwrap();
        }
        event_loop.run_app(&mut me).unwrap();
    }

    fn launch_game(
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) {
        let egui_context = egui::Context::default();
        let menu_state = MenuState::new(environment.clone(), rom_manager.clone());

        setup_theme(&egui_context);

        let mut me = Self {
            windowing: None,
            rom_manager,
            environment,
            egui_context,
            menu_state,
            audio: CpalAudio::default(),
            timing_tracker: TimingTracker::default(),
            gamepad_mapping: HashMap::new(),
            mode: RuntimeMode::Pending {
                game_system,
                user_specified_roms,
            },
            gui_active: true,
        };

        let event_loop = EventLoop::with_user_event().build().unwrap();
        {
            let event_loop_proxy = event_loop.create_proxy();
            std::thread::Builder::new()
                .name("gamepad".to_string())
                .spawn(move || {
                    tracing::debug!("Starting up gamepad thread");

                    gamepad_task(event_loop_proxy);

                    tracing::debug!("Shutting down gamepad thread");
                })
                .unwrap();
        }

        event_loop.run_app(&mut me).unwrap();
    }
}

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

pub enum RuntimeBoundMessage {
    Input {
        id: GamepadId,
        input: Input,
        state: InputState,
    },
}
