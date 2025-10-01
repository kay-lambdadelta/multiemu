use clap::Subcommand;
use multiemu::{environment::Environment, rom::RomMetadata};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Clone, Debug, Subcommand)]
pub enum LogiqxAction {
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
    },
}

pub fn database_logiqx_import(
    files: Vec<PathBuf>,
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rom_manager = Arc::new(RomMetadata::new(environment).unwrap());

    files.into_par_iter().try_for_each(|path| {
        let file = BufReader::new(File::open(&path)?);

        crate::logiqx::import(&rom_manager, file)
    })?;

    Ok(())
}
