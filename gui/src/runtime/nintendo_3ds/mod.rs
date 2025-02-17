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
    fn launch_gui(&mut self) {
        todo!()
    }

    fn launch_game(
        &mut self,
        user_specified_roms: Vec<RomId>,
        forced_game_system: Option<GameSystem>,
    ) {
        todo!()
    }
}
