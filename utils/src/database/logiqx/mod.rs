use clap::Subcommand;
use multiemu_config::Environment;
use multiemu_rom::info::RomInfo;
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use name::NameMetadataExtractor;
use parser::Datafile;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::BTreeSet;
use std::str::FromStr;
use std::{error::Error, fs::File, io::BufReader, path::PathBuf};

mod name;
mod parser;

#[derive(Clone, Debug, Subcommand)]
pub enum LogiqxAction {
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
    },
}

pub fn database_logiqx_import(files: Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let environment = Environment::load()?;

    let rom_manager = RomManager::new(Some(&environment.database_file))?;

    files
        .into_par_iter()
        .try_for_each(|path| {
            let file = BufReader::new(File::open(&path)?);

            // Parse XML based data file
            let data_file: Datafile = match quick_xml::de::from_reader(file) {
                Ok(file) => file,
                Err(err) => {
                    tracing::error!(
                        "Failed to parse XML logiqx database {}: {}",
                        path.display(),
                        err
                    );
                    return Ok(());
                }
            };

            tracing::info!(
                "Found {} entries in logiqx database {} for the system {}",
                data_file.machine.len(),
                path.display(),
                data_file.header.name
            );

            let database_transaction = rom_manager.rom_information.begin_write()?;
            let mut database_table = database_transaction.open_table(ROM_INFORMATION_TABLE)?;

            for mut entry in data_file.machine {
                let mut dependencies = BTreeSet::default();

                // Extract dependency roms
                for rom in entry.rom.drain(1..) {
                    let mut languages = BTreeSet::default();

                    if let Ok(name_metadata) = NameMetadataExtractor::from_str(&rom.name) {
                        languages.extend(name_metadata.languages);
                    }

                    let info = RomInfo {
                        name: rom.name,
                        system: data_file.header.name,
                        languages,
                        dependencies: BTreeSet::default(),
                    };

                    tracing::debug!("Full ROM info: {:#?}", info);

                    database_table.insert(rom.id, info)?;
                    dependencies.insert(rom.id);
                }

                let rom = entry.rom.remove(0);
                let mut languages = BTreeSet::default();

                if let Ok(name_metadata) = NameMetadataExtractor::from_str(&rom.name) {
                    languages.extend(name_metadata.languages);
                }

                let info = RomInfo {
                    name: rom.name,
                    system: data_file.header.name,
                    languages,
                    dependencies,
                };

                tracing::debug!("Full ROM info: {:#?}", info);

                database_table.insert(rom.id, info)?;
            }

            drop(database_table);
            database_transaction.commit()?;

            Ok(())
        })
        .map_err(|err: Box<dyn Error + Send + Sync>| err as Box<dyn Error>)?;

    Ok(())
}
