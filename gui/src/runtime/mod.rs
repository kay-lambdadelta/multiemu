use multiemu_config::Environment;
use multiemu_machine::display::backend::RenderBackend;
use multiemu_rom::id::RomId;
use multiemu_rom::manager::RomManager;
use multiemu_rom::system::GameSystem;
use std::sync::{Arc, RwLock};

#[cfg(platform_desktop)]
pub mod desktop;
#[cfg(platform_desktop)]
pub use desktop::renderer::software::SoftwareRenderingRuntime;

#[cfg(platform_3ds)]
pub mod nintendo_3ds;
#[cfg(platform_3ds)]
pub use nintendo_3ds::renderer::software::SoftwareRenderingRuntime;

/// A runtime for a given platform
pub trait Runtime<R: RenderBackend> {
    fn new(
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;

    fn launch_gui(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn launch_game(
        &mut self,
        game_system: GameSystem,
        user_specified_roms: Vec<RomId>,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
