use std::{fs::File, io::BufReader, path::PathBuf};

use clap::Subcommand;
use fluxemu_frontend::environment::Environment;
use fluxemu_runtime::program::ProgramManager;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[derive(Clone, Debug, Subcommand)]
pub enum LogiqxAction {
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
    },
}

pub fn database_logiqx_import(
    files: Vec<PathBuf>,
    environment: Environment,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let program_manager = ProgramManager::new(
        &environment.database_location,
        &environment.rom_store_directory,
    )
    .unwrap();

    files.into_par_iter().try_for_each(|path| {
        let file = BufReader::new(File::open(&path)?);

        crate::logiqx::import(&program_manager, file)
    })?;

    Ok(())
}
