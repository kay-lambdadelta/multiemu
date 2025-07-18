//! A multisystem hardware emulator

#![windows_subsystem = "windows"]
#![allow(clippy::arc_with_non_send_sync)]

use crate::{backend::software::SoftwareGraphicsRuntime, windowing::DesktopPlatform};
use clap::Parser;
use cli::{Cli, CliAction};
use multiemu_config::{ENVIRONMENT_LOCATION, Environment};
use multiemu_frontend::PlatformExt;
use multiemu_graphics::software::Software;
use multiemu_rom::{ROM_INFORMATION_TABLE, RomId, RomManager};
use multiemu_runtime::UserSpecifiedRoms;
use multiemu_save::{SaveManager, SnapshotManager};
use std::{
    borrow::Cow,
    fs::{File, create_dir_all},
    ops::Deref,
    sync::{Arc, RwLock},
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Layer, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

mod audio;
mod backend;
mod build_machine;
mod cli;
mod input;
mod windowing;

fn main() {
    // Set our current thread as our main thread
    multiemu_runtime::utils::set_main_thread();

    create_dir_all(multiemu_config::STORAGE_DIRECTORY.deref()).unwrap();

    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment = match Environment::load(environment_file) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!("Failed to load environment: {}", err);
            Environment::default()
        }
    };

    let file = File::create(&environment.log_location.0).expect("Failed to create log file");
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

    let rom_manager = Arc::new(
        RomManager::new(
            Some(environment.database_location.0.clone()),
            Some(environment.rom_store_directory.0.clone()),
        )
        .unwrap(),
    );
    let save_manager = Arc::new(SaveManager::new(Some(environment.save_directory.0.clone())));
    let snapshot_manager = Arc::new(SnapshotManager::new(Some(
        environment.snapshot_directory.0.clone(),
    )));

    let environment = Arc::new(RwLock::new(environment));

    let cli = Cli::parse();

    // TODO: Move this somewhere else
    if let Some(action) = cli.action {
        let (system, user_specified_roms) = match action {
            CliAction::Run {
                mut roms,
                forced_system,
            } => {
                let system = forced_system.unwrap_or_else(|| {
                    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
                    let database_table = database_transaction
                        .open_multimap_table(ROM_INFORMATION_TABLE)
                        .unwrap();
                    let rom_info = database_table
                        .get(&roms[0])
                        .unwrap()
                        .next()
                        .unwrap()
                        .unwrap()
                        .value();
                    rom_info.system
                });
                let main_rom = roms.remove(0);

                (
                    system,
                    UserSpecifiedRoms {
                        main: main_rom,
                        sub: Cow::Owned(roms),
                    },
                )
            }
            CliAction::RunExternal {
                roms,
                forced_system,
            } => {
                let mut roms: Vec<RomId> = roms
                    .into_iter()
                    .map(|rom| rom_manager.identify_rom(rom).unwrap().unwrap())
                    .collect();

                let system = forced_system.unwrap_or_else(|| {
                    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
                    let database_table = database_transaction
                        .open_multimap_table(ROM_INFORMATION_TABLE)
                        .unwrap();
                    let rom_info = database_table
                        .get(&roms[0])
                        .unwrap()
                        .next()
                        .unwrap()
                        .unwrap()
                        .value();
                    rom_info.system
                });

                let main_rom = roms.remove(0);

                (
                    system,
                    UserSpecifiedRoms {
                        main: main_rom,
                        sub: Cow::Owned(roms),
                    },
                )
            }
        };

        let api = environment.read().unwrap().graphics_setting.api;

        match api {
            multiemu_config::graphics::GraphicsApi::Software => {
                DesktopPlatform::<Software, SoftwareGraphicsRuntime>::run_with_machine(
                    environment.clone(),
                    rom_manager.clone(),
                    save_manager.clone(),
                    snapshot_manager.clone(),
                    build_machine::get_software_factories(),
                    system,
                    user_specified_roms,
                )
                .unwrap();
            }
            #[cfg(feature = "vulkan")]
            multiemu_config::graphics::GraphicsApi::Vulkan => {
                use crate::backend::vulkan::VulkanGraphicsRuntime;
                use multiemu_graphics::vulkan::Vulkan;

                DesktopPlatform::<Vulkan, VulkanGraphicsRuntime>::run_with_machine(
                    environment.clone(),
                    rom_manager.clone(),
                    save_manager.clone(),
                    snapshot_manager.clone(),
                    build_machine::get_vulkan_factories(),
                    system,
                    user_specified_roms,
                )
                .unwrap();
            }
            _ => todo!(),
        }

        return;
    }

    let api = environment.read().unwrap().graphics_setting.api;

    match api {
        multiemu_config::graphics::GraphicsApi::Software => {
            DesktopPlatform::<Software, SoftwareGraphicsRuntime>::run(
                environment.clone(),
                rom_manager.clone(),
                save_manager.clone(),
                snapshot_manager.clone(),
                build_machine::get_software_factories(),
            )
            .unwrap();
        }
        #[cfg(feature = "vulkan")]
        multiemu_config::graphics::GraphicsApi::Vulkan => {
            use crate::backend::vulkan::VulkanGraphicsRuntime;
            use multiemu_graphics::vulkan::Vulkan;

            DesktopPlatform::<Vulkan, VulkanGraphicsRuntime>::run(
                environment.clone(),
                rom_manager.clone(),
                save_manager.clone(),
                snapshot_manager.clone(),
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
