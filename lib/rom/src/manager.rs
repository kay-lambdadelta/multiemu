use crate::{id::RomId, info::RomInfo, system::System};
use redb::{
    Database, MultimapTableDefinition, ReadableDatabase, ReadableMultimapTable,
    backends::InMemoryBackend,
};
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    fs::{self, File, create_dir_all},
    path::{Path, PathBuf},
    str::FromStr,
    sync::RwLock,
};

/// Definition of the rom information table
pub const ROM_INFORMATION_TABLE: MultimapTableDefinition<RomId, RomInfo> =
    MultimapTableDefinition::new("rom_information");

#[derive(Debug, Clone)]
/// The location of a loaded ROM
pub enum LoadedRomLocation {
    /// The rom is in the emulators internal store named by its sha1
    Internal,
    /// The rom is somewhere else on disk
    External(PathBuf),
}

#[derive(Debug)]
/// The ROM manager which contains the database and information about the roms that were loaded
pub struct RomMetadata {
    /// [redb] database representing the ROM information
    pub rom_information: Database,
    /// The roms that the emulator is aware of location
    pub loaded_roms: RwLock<BTreeMap<RomId, LoadedRomLocation>>,
    rom_store: Option<PathBuf>,
}

impl RomMetadata {
    /// Opens and loads the default database
    pub fn new(
        database: Option<PathBuf>,
        rom_store: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Loading ROM database at {:?}", database);

        #[cfg(not(miri))]
        let rom_information = if let Some(path) = database {
            let _ = create_dir_all(path.parent().unwrap());

            Database::builder().create(path)?
        } else {
            Database::builder().create_with_backend(InMemoryBackend::default())?
        };

        #[cfg(miri)]
        let mut rom_information =
            Database::builder().create_with_backend(InMemoryBackend::default())?;

        #[cfg(not(miri))]
        {
            use std::sync::RwLock;

            let mut loaded_roms = BTreeMap::new();
            let mut database_transaction = rom_information.begin_write()?;
            database_transaction.set_quick_repair(true);
            database_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;
            database_transaction.commit()?;

            if let Some(rom_store) = &rom_store {
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
                rom_store,
            })
        }

        #[cfg(miri)]
        {
            Ok(Self {
                rom_information,
                loaded_roms: RwLock::default(),
                rom_store,
            })
        }
    }

    /// Imports a arbitary database into the internal database
    #[allow(clippy::result_large_err)]
    pub fn load_database(&self, path: impl AsRef<Path>) -> Result<(), redb::Error> {
        let path = path.as_ref();

        let database = Database::builder().open(path)?;
        let external_database_transaction = database.begin_read()?;
        let external_database_table =
            external_database_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;

        for item in external_database_table.iter()? {
            let (rom_id, rom_infos) = item?;

            let internal_database_transaction = self.rom_information.begin_write()?;
            let mut internal_database_table =
                internal_database_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;

            for rom_info in rom_infos.into_iter() {
                let rom_info = rom_info?;

                internal_database_table
                    .insert(rom_id.value(), rom_info.value())
                    .unwrap();
            }

            drop(internal_database_table);
            internal_database_transaction.commit()?;
        }

        Ok(())
    }

    /// Opens a ROM, giving a warning or panicking in the case that the requirement is not met
    ///
    /// Components should use this instead of directly opening ROM files
    pub fn open(&self, id: RomId, requirement: RomRequirement) -> Option<File> {
        if let Some(path) = self.get_rom_path(id) {
            if path.is_file() {
                return Some(File::open(path).unwrap());
            } else {
                tracing::error!("ROM {} is not a file", id);
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
        let rom_path = rom.as_ref();

        if !rom_path.is_file() {
            tracing::error!("ROM {} is not a file", rom_path.display());

            return Ok(None);
        }

        let file = File::open(rom_path)?;
        let rom_id = RomId::calculate_id(file)?;

        let write_transaction = self.rom_information.begin_write()?;
        let mut table = write_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;

        // Determine the game system
        let game_system = table
            .get(rom_id)
            .ok()
            .and_then(|info| {
                info.into_iter()
                    .next()
                    .and_then(|entry| entry.ok().map(|v| v.value().system()))
            })
            .or_else(|| System::guess(rom_path));

        if let Some(game_system) = game_system {
            // Update the ROM location
            self.loaded_roms
                .write()
                .unwrap()
                .insert(rom_id, LoadedRomLocation::External(rom_path.to_path_buf()));

            // Add a stub entry to the database if it doesn't exist
            if table.get(rom_id)?.is_empty() {
                tracing::info!(
                    "Adding basic ROM definition for {} to database (ID: {})",
                    rom_path.display(),
                    rom_id
                );

                table.insert(
                    rom_id,
                    RomInfo::V0 {
                        name: rom_path
                            .with_extension("")
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        path: vec![
                            rom_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        ],
                        system: game_system,
                        languages: HashSet::new(),
                        regions: HashSet::new(),
                    },
                )?;
            }

            drop(table);
            write_transaction.commit()?;
            return Ok(Some(rom_id));
        }

        tracing::error!("Could not identify ROM at {}", rom_path.display());
        Ok(None)
    }

    /// Get the path of a ROM on disk, if we have it
    pub fn get_rom_path(&self, id: RomId) -> Option<PathBuf> {
        if let Some(path) = self.loaded_roms.read().unwrap().get(&id) {
            match path {
                LoadedRomLocation::Internal => {
                    return Some(self.rom_store.as_ref().unwrap().join(id.to_string()));
                }
                LoadedRomLocation::External(path) => {
                    return Some(path.clone());
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The requirement of a ROM as pertains to a component attempting to load it
pub enum RomRequirement {
    /// Ok to boot machine without this ROM but runtime failure can occur without it
    Sometimes,
    /// Machine will boot emulating this ROM
    Optional,
    /// Machine can not boot without this ROM
    Required,
}
