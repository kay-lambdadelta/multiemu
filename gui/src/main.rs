//! A multisystem hardware emulator

use crate::runtime::{PlatformRuntime, Runtime, SoftwareRenderingRuntime};
use multiemu_config::graphics::GraphicsApi;
use multiemu_config::Environment;
use multiemu_rom::manager::RomManager;
use std::sync::{Arc, RwLock};

mod build_machine;
mod cli;
mod gui;
mod rendering_backend;
mod runtime;
mod timing_tracker;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("MultiEMU v{}", env!("CARGO_PKG_VERSION"));

    let environment = Environment::load().expect("Could not parse global config");
    let rom_manager = Arc::new(RomManager::new(Some(&environment.database_file)).unwrap());
    let graphics_api = environment.graphics_setting.api;
    let environment = Arc::new(RwLock::new(environment));

    match graphics_api {
        GraphicsApi::Software => {
            PlatformRuntime::<SoftwareRenderingRuntime>::launch_gui(rom_manager, environment);
        }
        #[cfg(all(feature = "vulkan", platform_desktop))]
        GraphicsApi::Vulkan => {
            use runtime::desktop::renderer::vulkan::VulkanRenderingRuntime;

            PlatformRuntime::<VulkanRenderingRuntime>::launch_gui(rom_manager, environment);
        }
        #[cfg(all(feature = "opengl", platform_desktop))]
        GraphicsApi::OpenGl => {
            use runtime::desktop::renderer::vulkan::VulkanRenderingRuntime;

            PlatformRuntime::<VulkanRenderingRuntime>::launch_gui(rom_manager, environment);
        }
    }
}
