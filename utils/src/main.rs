use crate::database::DatabaseAction;
use crate::database::logiqx::LogiqxAction;
use crate::database::native::NativeAction;
use crate::rom::RomAction;
use clap::Parser;

mod convert;
mod database;
mod rom;

#[derive(Clone, Parser)]
pub enum Cli {
    #[clap(subcommand)]
    Database(DatabaseAction),
    #[clap(subcommand)]
    Rom(RomAction),
}

fn main() {
    tracing_subscriber::fmt::init();

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
