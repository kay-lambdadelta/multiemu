use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use std::sync::{Arc, RwLock};

mod main_loop;

#[cfg(platform_desktop)]
pub mod desktop;

#[cfg(platform_desktop)]
pub use desktop::renderer::software::SoftwareRenderingRuntime;
#[cfg(platform_desktop)]
pub use desktop::PlatformRuntime;
use multiemu_config::Environment;
use multiemu_machine::display::RenderBackend;
use multiemu_rom::system::GameSystem;

#[cfg(platform_3ds)]
pub mod nintendo_3ds;
#[cfg(platform_3ds)]
pub use nintendo_3ds::renderer::software::SoftwareRenderingRuntime;
#[cfg(platform_3ds)]
pub use nintendo_3ds::PlatformRuntime;

pub trait Runtime<R: RenderBackend> {
    fn launch_gui(rom_manager: Arc<RomManager>, environment: Arc<RwLock<Environment>>);
    fn launch_game(
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    );
}
