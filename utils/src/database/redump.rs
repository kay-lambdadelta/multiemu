use clap::Subcommand;
use multiemu_config::Environment;
use multiemu_rom::{
    manager::RomManager,
    system::{GameSystem, NintendoSystem},
};
use std::{
    error::Error,
    io::{BufReader, Seek},
    sync::Arc,
};
use strum::{Display, EnumIter};
use zip::ZipArchive;

const BASE_URL: &str = "http://redump.org/datfile/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter, Display)]
pub enum RedumpSystem {
    Gc,
    Wii,
}

impl TryFrom<GameSystem> for RedumpSystem {
    type Error = ();

    fn try_from(value: GameSystem) -> Result<Self, Self::Error> {
        match value {
            GameSystem::Nintendo(NintendoSystem::GameCube) => Ok(Self::Gc),
            GameSystem::Nintendo(NintendoSystem::Wii) => Ok(Self::Wii),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum RedumpAction {
    Download {
        #[clap(required=true, num_args=1..)]
        systems: Vec<GameSystem>,
    },
    DownloadAll,
}

pub fn database_redump_download(
    systems: impl IntoIterator<Item = GameSystem>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let environment = Environment::load()?;
    let rom_manager = Arc::new(
        RomManager::new(
            Some(&environment.database_file),
            Some(&environment.roms_directory),
        )
        .unwrap(),
    );
    for system in systems {
        if let Ok(redump_system) = RedumpSystem::try_from(system) {
            tracing::info!("Downloading redump dat for system {}", system);

            let url = format!("{}/{}", BASE_URL, redump_system.to_string().to_lowercase());

            let mut temp_file = tempfile::tempfile()?;
            // Download to temp file
            std::io::copy(
                &mut ureq::get(url.to_string()).call()?.into_body().as_reader(),
                &mut temp_file,
            )?;

            temp_file.seek(std::io::SeekFrom::Start(0))?;
            let mut archive = ZipArchive::new(temp_file).unwrap();

            for index in 0..archive.len() {
                let file = BufReader::new(archive.by_index(index)?);

                crate::logiqx::import(&rom_manager, file)?;
            }
        }
    }

    Ok(())
}
