use crate::rendering_backend::RenderingBackendState;
use crate::runtime::Runtime;
use crossbeam::channel::Sender;
use main_loop::Message;
use multiemu_config::Environment;
use multiemu_machine::display::RenderBackend;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::GameSystem;
use std::sync::{Arc, Mutex, RwLock};
use winit::{event_loop::EventLoop, window::Window};

mod gamepad;
mod keyboard;
mod main_loop;
pub mod renderer;
mod windowing;

struct WindowingContext<RS: RenderingBackendState> {
    egui_winit: Arc<Mutex<egui_winit::State>>,
    display_api_handle: RS::DisplayApiHandle,
    runtime_channel: Sender<Message>,
}

pub struct PlatformRuntime<RS: RenderingBackendState> {
    windowing: Option<WindowingContext<RS>>,
    pending_machine: Option<(GameSystem, Vec<RomId>)>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
}

impl<R: RenderBackend, RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>>
    Runtime<R> for PlatformRuntime<RS>
{
    fn launch_gui(rom_manager: Arc<RomManager>, environment: Arc<RwLock<Environment>>) {
        let mut me = Self {
            windowing: None,
            pending_machine: None,
            rom_manager,
            environment,
        };

        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut me).unwrap();
    }

    fn launch_game(
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) {
        let mut me = Self {
            windowing: None,
            pending_machine: Some((game_system, user_specified_roms)),
            rom_manager,
            environment,
        };

        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut me).unwrap();
    }
}
