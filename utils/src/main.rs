use crate::{
    database::{DatabaseAction, logiqx::LogiqxAction, native::NativeAction},
    rom::RomAction,
};
use clap::Parser;
use database::redump::RedumpAction;
use multiemu_rom::system::GameSystem;
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

    match args {
        Cli::Database(DatabaseAction::Native {
            action: NativeAction::Import { paths },
        }) => {
            database::native::database_native_import(paths).unwrap();
        }
        Cli::Database(DatabaseAction::Native {
            action: NativeAction::FuzzySearch { search, similarity },
        }) => database::native::database_native_fuzzy_search(search, similarity).unwrap(),
        Cli::Database(DatabaseAction::Logiqx {
            action: LogiqxAction::Import { paths },
        }) => {
            database::logiqx::database_logiqx_import(paths).unwrap();
        }
        Cli::Database(DatabaseAction::Redump {
            action: RedumpAction::Download { systems },
        }) => {
            database::redump::database_redump_download(systems).unwrap();
        }
        Cli::Database(DatabaseAction::Redump {
            action: RedumpAction::DownloadAll,
        }) => {
            database::redump::database_redump_download(GameSystem::iter()).unwrap();
        }
        Cli::Database(DatabaseAction::ScreenScraper {}) => {}
        Cli::Rom(RomAction::Import { paths, symlink }) => {
            rom::import::rom_import(paths, symlink).unwrap();
        }
        Cli::Rom(RomAction::Export {
            path,
            symlink,
            style,
        }) => {
            rom::export::rom_export(path, symlink, style).unwrap();
        }
    }
}
