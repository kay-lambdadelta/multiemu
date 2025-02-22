use crate::{id::RomId, info::RomInfo};
use redb::{Database, ReadableTable, TableDefinition, backends::InMemoryBackend};
use std::{
    error::Error,
    fmt::Debug,
    fs::{File, create_dir_all},
    path::{Path, PathBuf},
};

pub const ROM_INFORMATION_TABLE: TableDefinition<RomId, RomInfo> =
    TableDefinition::new("rom_information");

#[derive(Debug, Clone)]
pub enum LoadedRomLocation {
    Internal,
    External(PathBuf),
}

#[derive(Debug)]
pub struct RomManager {
    pub rom_information: Database,
    pub loaded_roms: scc::HashMap<RomId, LoadedRomLocation>,
}

impl RomManager {
    /// Opens and loads the default database
    pub fn new(database: Option<&Path>) -> Result<Self, Box<dyn Error>> {
        tracing::info!("Loading ROM database at {:?}", database);

        let rom_information = if let Some(path) = database {
            let _ = create_dir_all(path.parent().unwrap());

            Database::builder().create(path)?
        } else {
            Database::builder().create_with_backend(InMemoryBackend::default())?
        };

        let database_transaction = rom_information.begin_write()?;
        database_transaction.open_table(ROM_INFORMATION_TABLE)?;
        database_transaction.commit()?;

        Ok(Self {
            rom_information,
            loaded_roms: scc::HashMap::new(),
        })
    }

    pub fn load_database(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = path.as_ref();

        if !path.is_file() {
            return Err("Path is not a file".into());
        }

        let database = Database::builder().open(path)?;
        let external_database_transaction = database.begin_read()?;
        let external_database_table =
            external_database_transaction.open_table(ROM_INFORMATION_TABLE)?;

        for item in external_database_table.iter()? {
            let (rom_id, rom_info) = item?;

            let internal_database_transaction = self.rom_information.begin_write()?;
            let mut internal_database_table =
                internal_database_transaction.open_table(ROM_INFORMATION_TABLE)?;
            internal_database_table.insert(rom_id.value(), rom_info.value())?;
            drop(internal_database_table);
            internal_database_transaction.commit()?;
        }

        Ok(())
    }

    /// Components should use this function to load roms for themselves
    pub fn open(&self, id: RomId, requirement: RomRequirement) -> Option<File> {
        if let Some(path) = self.loaded_roms.get(&id) {
            match path.get() {
                LoadedRomLocation::Internal => {}
                LoadedRomLocation::External(path) => {
                    return Some(File::open(path).unwrap());
                }
            }
        }

        match requirement {
            RomRequirement::Sometimes => {
                tracing::warn!(
                    "Could not find ROM {} for machine, machine will continue in a degraded state",
                    id
                );
            }
            RomRequirement::Optional => {
                tracing::info!(
                    "Could not find ROM {} for machine, but it's optional for runtime",
                    id
                );
            }
            RomRequirement::Required => {
                tracing::error!("ROM {} is required for machine, but not found", id);
            }
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RomRequirement {
    /// Ok to boot machine without this ROM but runtime failure can occur without it
    Sometimes,
    /// Machine will boot emulating this ROM
    Optional,
    /// Machine can not boot without this ROM
    Required,
}
