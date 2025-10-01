use crate::{
    database::{DatabaseAction, logiqx::LogiqxAction, native::NativeAction},
    rom::RomAction,
};
use clap::Parser;
use database::redump::RedumpAction;
use multiemu::{environment::ENVIRONMENT_LOCATION, rom::System};
use std::{
    fs::File,
    ops::Deref,
    sync::{Arc, RwLock},
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod convert;
mod database;
mod logiqx;
mod rom;

#[derive(Clone, Parser)]
pub enum Cli {
    #[clap(subcommand)]
    Database(DatabaseAction),
    #[clap(subcommand)]
    Rom(RomAction),
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let args = Cli::parse();

    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment = Arc::new(RwLock::new(
        ron::de::from_reader(environment_file).unwrap_or_default(),
    ));

    match args {
        Cli::Database(DatabaseAction::Native {
            action: NativeAction::Import { paths },
        }) => {
            database::native::database_native_import(paths, environment).unwrap();
        }
        Cli::Database(DatabaseAction::Native {
            action: NativeAction::FuzzySearch { search, similarity },
        }) => {
            database::native::database_native_fuzzy_search(search, similarity, environment).unwrap()
        }
        Cli::Database(DatabaseAction::Logiqx {
            action: LogiqxAction::Import { paths },
        }) => {
            database::logiqx::database_logiqx_import(paths, environment).unwrap();
        }
        Cli::Database(DatabaseAction::Redump {
            action: RedumpAction::Download { systems },
        }) => {
            database::redump::database_redump_download(systems, environment).unwrap();
        }
        Cli::Database(DatabaseAction::Redump {
            action: RedumpAction::DownloadAll,
        }) => {
            database::redump::database_redump_download(System::iter(), environment).unwrap();
        }
        Cli::Database(DatabaseAction::ScreenScraper {}) => {}
        Cli::Rom(RomAction::Import { paths, symlink }) => {
            rom::import::rom_import(paths, symlink, environment).unwrap();
        }
        Cli::Rom(RomAction::Export {
            path,
            symlink,
            style,
        }) => {
            rom::export::rom_export(path, symlink, style, environment).unwrap();
        }
        Cli::Rom(RomAction::Verify) => {
            rom::verify::rom_verify(environment).unwrap();
        }
    }
}
