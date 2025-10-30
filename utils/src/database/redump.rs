use clap::Subcommand;
use multiemu_runtime::{
    environment::Environment,
    program::{MachineId, NintendoSystem, ProgramManager, SegaSystem, SonySystem},
};
use std::{
    error::Error,
    io::{BufReader, Seek},
    sync::{Arc, RwLock},
};
use strum::{Display, EnumIter};
use zip::ZipArchive;

const BASE_URL: &str = "http://redump.org/datfile/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter, Display)]
pub enum RedumpSystem {
    Gc,
    Wii,
    Psx,
    Ps2,
    Ps3,
    Psp,
    Mcd,
}

impl TryFrom<MachineId> for RedumpSystem {
    type Error = ();

    fn try_from(value: MachineId) -> Result<Self, Self::Error> {
        match value {
            MachineId::Nintendo(NintendoSystem::GameCube) => Ok(Self::Gc),
            MachineId::Nintendo(NintendoSystem::Wii) => Ok(Self::Wii),
            MachineId::Sony(SonySystem::Playstation) => Ok(Self::Psx),
            MachineId::Sony(SonySystem::Playstation2) => Ok(Self::Ps2),
            MachineId::Sony(SonySystem::Playstation3) => Ok(Self::Ps3),
            MachineId::Sony(SonySystem::PlaystationPortable) => Ok(Self::Psp),
            MachineId::Sega(SegaSystem::SegaCD) => Ok(Self::Mcd),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum RedumpAction {
    Download {
        #[clap(required=true, num_args=1..)]
        systems: Vec<MachineId>,
    },
    DownloadAll,
}

pub fn database_redump_download(
    systems: impl IntoIterator<Item = MachineId>,
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let program_manager = ProgramManager::new(environment).unwrap();

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

                crate::logiqx::import(&program_manager, file)?;
            }
        }
    }

    Ok(())
}
