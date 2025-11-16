use std::{
    fs::{File, create_dir_all},
    ops::Deref,
};

use clap::Parser;
use data_encoding::HEXLOWER_PERMISSIVE;
use database::redump::RedumpAction;
use multiemu_frontend::environment::{ENVIRONMENT_LOCATION, Environment, STORAGE_DIRECTORY};
use multiemu_runtime::program::MachineId;
use serde::{Deserialize, Deserializer};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::{
    database::{DatabaseAction, logiqx::LogiqxAction, native::NativeAction},
    rom::RomAction,
    search::SearchAction,
};

mod convert;
mod database;
mod logiqx;
mod patch;
mod rom;
mod search;

#[derive(Clone, Parser)]
pub enum Cli {
    #[clap(subcommand)]
    Database(DatabaseAction),
    #[clap(subcommand)]
    Rom(RomAction),
    #[clap(subcommand)]
    Search(SearchAction),
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

    create_dir_all(STORAGE_DIRECTORY.deref()).unwrap();
    let environment = File::open(ENVIRONMENT_LOCATION.deref())
        .ok()
        .and_then(|f| Environment::load(f).ok())
        .unwrap_or_default();

    if !ENVIRONMENT_LOCATION.is_file() {
        let mut environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
        environment.save(&mut environment_file).unwrap();
    }

    match args {
        Cli::Database(DatabaseAction::Native {
            action: NativeAction::Import { paths },
        }) => {
            database::native::database_native_import(paths, environment).unwrap();
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
            database::redump::database_redump_download(MachineId::iter(), environment).unwrap();
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
        Cli::Search(action) => search::search(environment, action).unwrap(),
    }
}

pub fn deserialize_hex_array<'de, D, const N: usize>(
    deserializer: D,
) -> Result<Option<[u8; N]>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|s| {
            let bytes = HEXLOWER_PERMISSIVE
                .decode(s.as_bytes())
                .map_err(serde::de::Error::custom)?;

            if bytes.len() != N {
                return Err(serde::de::Error::invalid_length(
                    bytes.len(),
                    &format!("{N}").as_str(),
                ));
            }

            Ok(bytes.try_into().unwrap())
        })
        .transpose()
}
