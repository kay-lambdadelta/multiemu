use clap::Subcommand;
use multiemu_config::Environment;
use multiemu_rom::manager::RomManager;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(Clone, Debug, Subcommand)]
pub enum LogiqxAction {
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
    },
}

pub fn database_logiqx_import(
    files: Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment = Environment::load()?;
    let rom_manager = RomManager::new(Some(&environment.database_file))?;

    files.into_par_iter().try_for_each(|path| {
        let file = BufReader::new(File::open(&path)?);

        crate::logiqx::import(&rom_manager, file)
    })?;

    Ok(())
}
