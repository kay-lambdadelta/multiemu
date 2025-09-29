use clap::Subcommand;
use multiemu_config::Environment;
use multiemu_rom::RomMetadata;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{fs::File, io::BufReader, path::PathBuf, sync::Arc};

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
    let rom_manager = Arc::new(
        RomMetadata::new(
            Some(environment.database_location.0.clone()),
            Some(environment.rom_store_directory.0.clone()),
        )
        .unwrap(),
    );

    files.into_par_iter().try_for_each(|path| {
        let file = BufReader::new(File::open(&path)?);

        crate::logiqx::import(&rom_manager, file)
    })?;

    Ok(())
}
