use crate::{
    database::{DatabaseAction, logiqx::LogiqxAction, native::NativeAction},
    rom::RomAction,
};
use clap::Parser;
use data_encoding::HEXLOWER_PERMISSIVE;
use database::redump::RedumpAction;
use multiemu_base::{
    environment::{ENVIRONMENT_LOCATION, STORAGE_DIRECTORY},
    program::MachineId,
};
use serde::{Deserialize, Deserializer};
use std::{
    fs::{File, create_dir_all},
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

    let _ = create_dir_all(STORAGE_DIRECTORY.deref());

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
                    &format!("{}", N).as_str(),
                ));
            }

            Ok(bytes.try_into().unwrap())
        })
        .transpose()
}
