//! A multisystem hardware emulator

#![windows_subsystem = "windows"]
#![allow(clippy::arc_with_non_send_sync)]

use crate::runtime::{Platform, SoftwareRenderingRuntime, state::MainRuntime};
use multiemu_config::{ENVIRONMENT_LOCATION, Environment, graphics::GraphicsApi};
use multiemu_rom::{
    id::RomId,
    manager::{ROM_INFORMATION_TABLE, RomManager},
};
use ron::ser::PrettyConfig;
use std::{
    fs::File,
    io::Write,
    sync::{Arc, RwLock},
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Layer, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

mod build_machine;
mod cli;
mod gui;
mod rendering_backend;
mod runtime;

#[cfg(all(
    any(target_family = "unix", target_os = "windows"),
    not(target_os = "horizon")
))]
fn main() {
    use std::ops::Deref;

    use clap::Parser;
    use cli::{Cli, CliAction};
    use runtime::desktop::windowing::DesktopPlatform;
    use tracing_subscriber::fmt::format::FmtSpan;

    // Set our current thread as our main thread
    multiemu_runtime::utils::set_main_thread();

    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

    let file = File::create(&environment.log_location.0).expect("Failed to create log file");
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
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
    let graphics_api = environment.graphics_setting.api;
    let environment = Arc::new(RwLock::new(environment));

    let cli = Cli::parse();

    // TODO: Move this somewhere else
    if let Some(action) = cli.action {
        let (game_system, user_specified_roms) = match action {
            CliAction::Run {
                roms,
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
                (system, roms)
            }
            CliAction::RunExternal {
                roms,
                forced_system,
            } => {
                let rom_ids: Vec<RomId> = roms
                    .into_iter()
                    .map(|rom| rom_manager.identify_rom(rom).unwrap().unwrap())
                    .collect();

                let game_system = forced_system.unwrap_or_else(|| {
                    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
                    let database_table = database_transaction
                        .open_multimap_table(ROM_INFORMATION_TABLE)
                        .unwrap();
                    let rom_info = database_table
                        .get(&rom_ids[0])
                        .unwrap()
                        .next()
                        .unwrap()
                        .unwrap()
                        .value();
                    rom_info.system
                });

                (game_system, rom_ids)
            }
        };

        match graphics_api {
            GraphicsApi::Software => {
                let runtime = MainRuntime::<SoftwareRenderingRuntime, _>::new_with_machine(
                    environment.clone(),
                    rom_manager.clone(),
                    build_machine::get_software_factories(),
                    game_system,
                    user_specified_roms.clone(),
                );

                DesktopPlatform::run(runtime).unwrap();
            }
            #[cfg(feature = "vulkan")]
            GraphicsApi::Vulkan => {
                use runtime::desktop::renderer::vulkan::VulkanRenderingRuntime;

                let runtime = MainRuntime::<VulkanRenderingRuntime, _>::new_with_machine(
                    environment.clone(),
                    rom_manager.clone(),
                    build_machine::get_vulkan_factories(),
                    game_system,
                    user_specified_roms.clone(),
                );

                DesktopPlatform::run(runtime).unwrap();
            }
            _ => unimplemented!(),
        }

        return;
    }

    match graphics_api {
        GraphicsApi::Software => {
            let runtime = MainRuntime::<SoftwareRenderingRuntime, _>::new(
                environment.clone(),
                rom_manager.clone(),
                build_machine::get_software_factories(),
            );

            DesktopPlatform::run(runtime).unwrap();
        }
        #[cfg(feature = "vulkan")]
        GraphicsApi::Vulkan => {
            use runtime::desktop::renderer::vulkan::VulkanRenderingRuntime;

            let runtime = MainRuntime::<VulkanRenderingRuntime, _>::new(
                environment.clone(),
                rom_manager.clone(),
                build_machine::get_vulkan_factories(),
            );

            DesktopPlatform::run(runtime).unwrap();
        }
        _ => unimplemented!(),
    }
}

fn create_filter() -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
}

fn write_environment(writer: impl Write, environment: &Environment) -> Result<(), ron::Error> {
    ron::Options::default().to_io_writer_pretty(
        writer,
        environment,
        PrettyConfig::new().struct_names(false),
    )
}
