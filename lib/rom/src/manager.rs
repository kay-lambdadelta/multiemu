use crate::{id::RomId, info::RomInfo, system::GameSystem};
use indexmap::IndexMap;
use redb::{Database, ReadableTable, TableDefinition, backends::InMemoryBackend};
use std::{
    collections::BTreeSet,
    fmt::Debug,
    fs::{self, File, create_dir_all},
    path::{Path, PathBuf},
    str::FromStr,
    sync::RwLock,
};

/// Definition of the rom information table
pub const ROM_INFORMATION_TABLE: TableDefinition<RomId, RomInfo> =
    TableDefinition::new("rom_information");

#[derive(Debug, Clone)]
pub enum LoadedRomLocation {
    /// The rom is in the emulators internal store named by its sha1
    Internal,
    /// The rom is somewhere else on disk
    External(PathBuf),
}

#[derive(Debug)]
/// The ROM manager which contains the database and information about the roms that were loaded
pub struct RomManager {
    pub rom_information: Database,
    pub loaded_roms: RwLock<IndexMap<RomId, LoadedRomLocation>>,
}

impl RomManager {
    /// Opens and loads the default database
    pub fn new(
        database: Option<&Path>,
        rom_store: Option<&Path>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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

        let mut loaded_roms = IndexMap::new();

        if let Some(rom_store) = rom_store {
            let _ = create_dir_all(rom_store);
            if rom_store.is_dir() {
                for file in fs::read_dir(rom_store)? {
                    let file = file?.file_name();

                    if let Some(file) = file
                        .clone()
                        .into_string()
                        .ok()
                        .and_then(|file| RomId::from_str(&file).ok())
                    {
                        loaded_roms.insert(file, LoadedRomLocation::Internal);
                    } else {
                        tracing::error!(
                            "Internal ROM store has a file thats name is not a valid ROM ID, please remove it: {:?}",
                            file
                        );
                    }
                }
            }
        }

        Ok(Self {
            rom_information,
            loaded_roms: RwLock::new(loaded_roms),
        })
    }

    /// Imports a arbitary database into the internal database
    pub fn load_database(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    pub fn open(
        &self,
        id: RomId,
        requirement: RomRequirement,
        internal_rom_store: impl AsRef<Path>,
    ) -> Option<File> {
        if let Some(path) = self.loaded_roms.read().unwrap().get(&id).cloned() {
            match path {
                LoadedRomLocation::Internal => {
                    let internal_rom_store = internal_rom_store.as_ref();
                    return Some(File::open(internal_rom_store.join(id.to_string())).unwrap());
                }
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

    /// Identifies a ROM and inserts data into the database for it if it can find it
    pub fn identify_rom(
        &self,
        rom: impl AsRef<Path>,
    ) -> Result<Option<RomId>, Box<dyn std::error::Error>> {
        let rom = rom.as_ref();

        if rom.is_file() {
            let file = File::open(rom).unwrap();
            let rom_id = RomId::from_read(file);

            let write_transaction = self.rom_information.begin_write().unwrap();
            let mut table = write_transaction.open_table(ROM_INFORMATION_TABLE).unwrap();

            // Try to figure out what kind of game is this
            if let Some(game_system) = table
                .get(rom_id)
                .unwrap()
                .map(|info| info.value().system)
                .or_else(|| GameSystem::guess(rom))
            {
                // Put its location in the store
                self.loaded_roms
                    .write()
                    .unwrap()
                    .insert(rom_id, LoadedRomLocation::External(rom.to_path_buf()));

                // Add a stub for it in the database
                if table.get(rom_id).unwrap().is_none() {
                    tracing::info!(
                        "Adding basic ROM definition for {} to database due to it being absent (its id is {})",
                        rom.display(),
                        rom_id
                    );

                    table
                        .insert(
                            rom_id,
                            RomInfo {
                                name: rom.file_name().unwrap().to_string_lossy().to_string(),
                                system: game_system,
                                languages: BTreeSet::default(),
                                dependencies: BTreeSet::default(),
                            },
                        )
                        .unwrap();
                }

                drop(table);
                write_transaction.commit().unwrap();

                return Ok(Some(rom_id));
            } else {
                tracing::error!("Could not identify ROM at {}", rom.display());
            }
        }

        Ok(None)
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
