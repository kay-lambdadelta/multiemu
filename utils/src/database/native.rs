use clap::Subcommand;
use multiemu_frontend::environment::Environment;
use multiemu_runtime::program::ProgramManager;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::PathBuf;

#[derive(Clone, Debug, Subcommand)]
pub enum NativeAction {
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
    },
}

pub fn database_native_import(
    paths: Vec<PathBuf>,
    environment: Environment,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let program_manager = ProgramManager::new(
        &environment.database_location,
        &environment.rom_store_directory,
    )
    .unwrap();

    paths
        .into_par_iter()
        .try_for_each(|path| program_manager.load_database(path))?;

    Ok(())
}
