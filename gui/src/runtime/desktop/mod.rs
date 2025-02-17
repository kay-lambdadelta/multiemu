use super::main_loop::Message;
use crate::rendering_backend::RenderingBackendState;
use crate::runtime::Runtime;
use crossbeam::channel::Sender;
use multiemu_config::Environment;
use multiemu_machine::display::RenderBackend;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::GameSystem;
use std::sync::{Arc, RwLock};
use winit::{event_loop::EventLoop, window::Window};

mod gamepad;
pub mod renderer;
mod windowing;

pub struct PlatformRuntime<RS: RenderingBackendState> {
    display_api_handle: Option<RS::DisplayApiHandle>,
    runtime_channel: Option<Sender<Message>>,
    pending_machine: Option<(GameSystem, Vec<RomId>)>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
}

impl<
        R: RenderBackend,
        RS: RenderingBackendState<DisplayApiHandle = Arc<Window>, RenderBackend = R>,
    > Runtime<R> for PlatformRuntime<RS>
{
    fn launch_gui(rom_manager: Arc<RomManager>, environment: Arc<RwLock<Environment>>) {
        let mut me = Self {
            display_api_handle: None,
            runtime_channel: None,
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
            display_api_handle: None,
            runtime_channel: None,
            pending_machine: Some((game_system, user_specified_roms)),
            rom_manager,
            environment,
        };

        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut me).unwrap();
    }
}
