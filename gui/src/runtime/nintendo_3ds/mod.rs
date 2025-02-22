use crate::{
    gui::menu::MenuState,
    rom::{id::RomId, system::GameSystem},
    runtime::launch::Runtime,
};
use ctru::prelude::{Apt, Gfx};
use std::rc::Rc;

pub struct PlatformRuntime {
    applet_service: Apt,
    graphics_service: Rc<Gfx>,
    menu_state: MenuState,
}

impl Default for PlatformRuntime {
    fn default() -> Self {
        Self {
            applet_service: Apt::new().unwrap(),
            graphics_service: Rc::new(Gfx::new().unwrap()),
            menu_state: MenuState::default(),
        }
    }
}

impl Runtime for PlatformRuntime {
    fn launch_gui(rom_manager: Arc<RomManager>, environment: Arc<RwLock<Environment>>) {}
    fn launch_game(
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) {
    }
}
