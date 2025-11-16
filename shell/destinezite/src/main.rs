//! A multisystem hardware emulator

#![windows_subsystem = "windows"]
#![allow(clippy::arc_with_non_send_sync)]

use std::{
    fs::{File, create_dir_all},
    ops::Deref,
    path::PathBuf,
};

use clap::Parser;
use cli::{Cli, CliAction};
use multiemu_frontend::{
    PlatformExt,
    environment::{ENVIRONMENT_LOCATION, Environment, STORAGE_DIRECTORY},
};
use multiemu_runtime::{graphics::software::Software, program::ProgramManager};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Layer, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

use crate::{
    backend::software::SoftwareGraphicsRuntime, input::DEFAULT_HOTKEYS, windowing::DesktopPlatform,
};

mod audio;
mod backend;
mod build_machine;
mod cli;
mod input;
mod windowing;

fn main() {
    create_dir_all(STORAGE_DIRECTORY.deref()).unwrap();

    let mut environment = File::open(ENVIRONMENT_LOCATION.deref())
        .ok()
        .and_then(|f| Environment::load(f).ok())
        .unwrap_or_default();

    if environment.hotkeys.is_empty() {
        environment.hotkeys = DEFAULT_HOTKEYS.clone();
    }

    if !ENVIRONMENT_LOCATION.is_file() {
        let mut environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
        environment.save(&mut environment_file).unwrap();
    }

    let file = File::create(&environment.log_location).expect("Failed to create log file");
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_thread_ids(true)
        .with_filter(create_filter());
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file)
        .with_ansi(false)
        .with_thread_ids(true)
        .with_filter(create_filter());

    // Combine the layers and set the global subscriber
    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();

    tracing::info!("MultiEMU v{}", env!("CARGO_PKG_VERSION"));

    let program_manager = ProgramManager::new(
        &environment.database_location,
        &environment.rom_store_directory,
    )
    .unwrap();

    let cli = Cli::parse();

    // TODO: Move this somewhere else
    if let Some(action) = cli.action {
        let program_specification = match action {
            CliAction::Run {
                roms,
                forced_machine_id,
            } => {
                let main_rom_as_path = PathBuf::from(roms[0].clone());
                let mut program_specification = if main_rom_as_path.is_file() {
                    // Interpret them all as paths

                    program_manager
                        .identify_program_from_paths(roms.into_iter().map(PathBuf::from))
                        .unwrap()
                        .unwrap()
                } else {
                    // program id launching needs to be done here
                    todo!()
                };

                if let Some(forced_machine_id) = forced_machine_id {
                    program_specification.id.machine = forced_machine_id;
                }

                program_specification
            }
        };

        match environment.graphics_setting.api {
            multiemu_frontend::environment::graphics::GraphicsApi::Software => {
                DesktopPlatform::<Software, SoftwareGraphicsRuntime>::run_with_program(
                    environment,
                    program_manager.clone(),
                    build_machine::get_software_factories(),
                    program_specification,
                )
                .unwrap();
            }
            #[cfg(feature = "vulkan")]
            multiemu_frontend::environment::graphics::GraphicsApi::Vulkan => {
                use multiemu_runtime::graphics::vulkan::Vulkan;

                use crate::backend::vulkan::VulkanGraphicsRuntime;

                DesktopPlatform::<Vulkan, VulkanGraphicsRuntime>::run_with_program(
                    environment,
                    program_manager.clone(),
                    build_machine::get_vulkan_factories(),
                    program_specification,
                )
                .unwrap();
            }
            _ => todo!(),
        }

        return;
    }

    match environment.graphics_setting.api {
        multiemu_frontend::environment::graphics::GraphicsApi::Software => {
            DesktopPlatform::<Software, SoftwareGraphicsRuntime>::run(
                environment,
                program_manager.clone(),
                build_machine::get_software_factories(),
            )
            .unwrap();
        }
        #[cfg(feature = "vulkan")]
        multiemu_frontend::environment::graphics::GraphicsApi::Vulkan => {
            use multiemu_runtime::graphics::vulkan::Vulkan;

            use crate::backend::vulkan::VulkanGraphicsRuntime;

            DesktopPlatform::<Vulkan, VulkanGraphicsRuntime>::run(
                environment,
                program_manager.clone(),
                build_machine::get_vulkan_factories(),
            )
            .unwrap();
        }
        _ => todo!(),
    }
}

fn create_filter() -> EnvFilter {
    EnvFilter::builder()
        .with_regex(true)
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
}
