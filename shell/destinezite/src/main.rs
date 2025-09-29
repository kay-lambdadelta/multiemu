//! A multisystem hardware emulator

#![windows_subsystem = "windows"]
#![allow(clippy::arc_with_non_send_sync)]

use crate::{backend::software::SoftwareGraphicsRuntime, windowing::DesktopPlatform};
use clap::Parser;
use cli::{Cli, CliAction};
use multiemu_config::{ENVIRONMENT_LOCATION, Environment};
use multiemu_frontend::PlatformExt;
use multiemu_graphics::software::Software;
use multiemu_rom::{ROM_INFORMATION_TABLE, RomId, RomInfo, RomMetadata};
use multiemu_runtime::{RomSpecification, UserSpecifiedRoms};
use redb::ReadableDatabase;
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

    let environment = File::open(ENVIRONMENT_LOCATION.deref())
        .ok()
        .and_then(|f| Environment::load(f).ok())
        .unwrap_or_default();

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
        RomMetadata::new(
            Some(environment.database_location.0.clone()),
            Some(environment.rom_store_directory.0.clone()),
        )
        .unwrap(),
    );
    let environment = Arc::new(RwLock::new(environment));

    let cli = Cli::parse();

    // TODO: Move this somewhere else
    if let Some(action) = cli.action {
        let user_specified_roms = match action {
            CliAction::Run {
                roms,
                forced_system,
            } => {
                let mut roms: Vec<_> = roms
                    .into_iter()
                    .map(|rom| {
                        let database_transaction =
                            rom_manager.rom_information.begin_read().unwrap();
                        let database_table = database_transaction
                            .open_multimap_table(ROM_INFORMATION_TABLE)
                            .unwrap();

                        RomSpecification {
                            id: rom,
                            identity: database_table
                                .get(&rom)
                                .unwrap()
                                .next()
                                .unwrap()
                                .unwrap()
                                .value(),
                        }
                    })
                    .collect();
                let mut main_rom = roms.remove(0);

                if let Some(forced_system) = forced_system {
                    match &mut main_rom.identity {
                        RomInfo::V0 { system, .. } => *system = forced_system,
                    }
                }

                UserSpecifiedRoms {
                    main: main_rom,
                    sub: Cow::Owned(roms),
                }
            }
            CliAction::RunExternal {
                roms,
                forced_system,
            } => {
                let roms: Vec<RomId> = roms
                    .into_iter()
                    .map(|rom| rom_manager.identify_rom(rom).unwrap().unwrap())
                    .collect();

                let mut roms: Vec<_> = roms
                    .into_iter()
                    .map(|rom| {
                        let database_transaction =
                            rom_manager.rom_information.begin_read().unwrap();
                        let database_table = database_transaction
                            .open_multimap_table(ROM_INFORMATION_TABLE)
                            .unwrap();

                        RomSpecification {
                            id: rom,
                            identity: database_table
                                .get(&rom)
                                .unwrap()
                                .next()
                                .unwrap()
                                .unwrap()
                                .value(),
                        }
                    })
                    .collect();
                let mut main_rom = roms.remove(0);

                if let Some(forced_system) = forced_system {
                    match &mut main_rom.identity {
                        RomInfo::V0 { system, .. } => *system = forced_system,
                    }
                }

                UserSpecifiedRoms {
                    main: main_rom,
                    sub: Cow::Owned(roms),
                }
            }
        };

        let api = environment.read().unwrap().graphics_setting.api;

        match api {
            multiemu_config::graphics::GraphicsApi::Software => {
                DesktopPlatform::<Software, SoftwareGraphicsRuntime>::run_with_machine(
                    environment.clone(),
                    rom_manager.clone(),
                    build_machine::get_software_factories(),
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
                    build_machine::get_vulkan_factories(),
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
