use crate::{
    environment::Environment,
    program::{MachineId, ProgramId, ProgramInfo, ProgramSpecification, RomId, info::Filesystem},
};
use redb::{
    Database, MultimapTableDefinition, ReadableDatabase, ReadableMultimapTable,
    backends::InMemoryBackend,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Debug,
    fs::{self, File, create_dir_all},
    io::Seek,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, RwLock},
};

/// Program id -> Program info mapping
pub const PROGRAM_INFORMATION_TABLE: MultimapTableDefinition<ProgramId, ProgramInfo> =
    MultimapTableDefinition::new("program_information");
/// Hash -> Program id reverse mapping
pub const HASH_ALIAS_TABLE: MultimapTableDefinition<RomId, ProgramId> =
    MultimapTableDefinition::new("hash_alias");

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
pub struct ProgramMetadata {
    /// [redb] database representing the ROM information
    pub database: Database,
    /// The roms that the emulator is aware of location
    pub loaded_roms: RwLock<BTreeMap<RomId, LoadedRomLocation>>,
    environment: Arc<RwLock<Environment>>,
}

impl ProgramMetadata {
    /// Opens and loads the default database
    pub fn new(
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let environment_guard = environment.read().unwrap();

        tracing::info!(
            "Loading ROM database at {:?}",
            environment_guard.database_location
        );
        let _ = create_dir_all(environment_guard.database_location.parent().unwrap());

        let rom_information =
            Database::builder().create(environment_guard.database_location.clone())?;

        let mut loaded_roms = BTreeMap::new();
        let mut database_transaction = rom_information.begin_write()?;
        database_transaction.set_quick_repair(true);
        database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;
        database_transaction.open_multimap_table(HASH_ALIAS_TABLE)?;
        database_transaction.commit()?;

        let _ = create_dir_all(&environment_guard.rom_store_directory);
        if environment_guard.rom_store_directory.is_dir() {
            for file in fs::read_dir(&environment_guard.rom_store_directory)? {
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

        drop(environment_guard);

        Ok(Self {
            database: rom_information,
            loaded_roms: RwLock::new(loaded_roms),
            environment,
        })
    }

    /// Opens a dumb in memory database
    pub fn new_test(
        environment: Arc<RwLock<Environment>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let environment_guard = environment.read().unwrap();

        let rom_information = Database::builder().create_with_backend(InMemoryBackend::default())?;

        let mut loaded_roms = BTreeMap::new();
        let database_transaction = rom_information.begin_write()?;
        database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;
        database_transaction.open_multimap_table(HASH_ALIAS_TABLE)?;
        database_transaction.commit()?;

        let _ = create_dir_all(&environment_guard.rom_store_directory);
        if environment_guard.rom_store_directory.is_dir() {
            for file in fs::read_dir(&environment_guard.rom_store_directory)? {
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

        drop(environment_guard);

        Ok(Self {
            database: rom_information,
            loaded_roms: RwLock::new(loaded_roms),
            environment,
        })
    }

    /// Imports a arbitary database into the internal database
    pub fn load_database(&self, path: impl AsRef<Path>) -> Result<(), redb::Error> {
        let path = path.as_ref();

        let database = Database::builder().open(path)?;
        let external_database_transaction = database.begin_read()?;
        let external_database_table =
            external_database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

        for item in external_database_table.iter()? {
            let (rom_id, rom_infos) = item?;

            let internal_database_transaction = self.database.begin_write()?;
            let mut internal_database_table =
                internal_database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

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

    /// Tries to see if some ROMS represents something in our database
    pub fn identify_program_from_paths<'a>(
        &'a self,
        roms: impl IntoIterator<Item = PathBuf> + 'a,
    ) -> Result<Option<ProgramSpecification>, Box<dyn std::error::Error>> {
        let rom_paths = Vec::from_iter(roms);
        let mut rom_files = HashMap::new();

        for path in rom_paths.iter() {
            let file = File::open(path)?;
            rom_files.insert(path, file);
        }

        let mut hashes = Vec::default();

        let mut loaded_roms_guard = self.loaded_roms.write().unwrap();

        for (rom_path, rom) in rom_files.iter_mut() {
            let sha1 = RomId::calculate_id(rom)?;
            rom.rewind()?;

            loaded_roms_guard.insert(sha1, LoadedRomLocation::External(rom_path.to_path_buf()));
            hashes.push(sha1);
        }

        if let Some(specification) = self.identify_program(hashes.clone())? {
            return Ok(Some(specification));
        }

        if let Some((path, machine_id)) = rom_paths.into_iter().find_map(|path| {
            let machine_id = MachineId::guess(&path);

            machine_id.map(|id| (path, id))
        }) && let Some(file_name) = path
            .file_name()
            .map(|string| string.to_string_lossy().to_string())
        {
            let extensionless_file_name = path
                .with_extension("")
                .file_name()
                .map(|string| string.to_string_lossy().to_string())
                .unwrap();

            return Ok(Some(ProgramSpecification {
                id: ProgramId {
                    machine: machine_id,
                    name: file_name.clone(),
                },
                info: ProgramInfo::V0 {
                    filesystem: Filesystem::Single {
                        rom_id: hashes[0],
                        file_name: file_name.clone(),
                    },
                    languages: BTreeSet::default(),
                    version: None,
                    names: BTreeSet::from_iter([extensionless_file_name]),
                },
            }));
        }

        Ok(None)
    }

    // Try to identify the game these roms belong to
    pub fn identify_program(
        &self,
        roms: impl IntoIterator<Item = RomId>,
    ) -> Result<Option<ProgramSpecification>, Box<dyn std::error::Error>> {
        let roms = Vec::from_iter(roms);
        let read_transaction = self.database.begin_read()?;
        let hash_alias_table = read_transaction.open_multimap_table(HASH_ALIAS_TABLE)?;
        let program_info_table = read_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

        Ok(roms
            .iter()
            .find_map(|rom| {
                hash_alias_table
                    .get(rom)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .find_map(|program_id| {
                        let program_id: ProgramId = program_id.value();

                        program_info_table
                            .get(&program_id)
                            .into_iter()
                            .flatten()
                            .flatten()
                            .find_map(|info| {
                                let info = info.value();

                                match info.filesystem() {
                                    Filesystem::Single {
                                        rom_id,
                                        file_name: _,
                                    } => {
                                        if roms.len() == 1 && roms[0] == *rom_id {
                                            return Some(ProgramSpecification {
                                                id: program_id.clone(),
                                                info,
                                            });
                                        }
                                    }
                                    // TODO: match on subpaths also
                                    Filesystem::Complex(fs) => {
                                        let found_all = roms.iter().all(|id| fs.contains_key(id));

                                        if found_all {
                                            return Some(ProgramSpecification {
                                                id: program_id.clone(),
                                                info,
                                            });
                                        }
                                    }
                                }

                                None
                            })
                    })
            })
            .or_else(|| {
                if roms.len() == 1
                    && let Some(rom_location) = self.loaded_roms.read().unwrap().get(&roms[0])
                {
                    let rom_id = roms[0];

                    let internal_path = self.get_rom_path(rom_id)?;

                    let name = match rom_location {
                        LoadedRomLocation::Internal => rom_id.to_string(),
                        LoadedRomLocation::External(path) => path
                            .with_extension("")
                            .file_name()
                            .map(|string| string.to_string_lossy().to_string())?,
                    };

                    let machine = MachineId::guess(internal_path)?;

                    Some(ProgramSpecification {
                        info: ProgramInfo::V0 {
                            names: BTreeSet::from_iter([name.clone()]),
                            filesystem: Filesystem::Single {
                                rom_id,
                                file_name: name.clone(),
                            },
                            languages: Default::default(),
                            version: None,
                        },
                        id: ProgramId { machine, name },
                    })
                } else {
                    None
                }
            }))
    }

    /// Get the path of a ROM on disk, if we have it
    pub fn get_rom_path(&self, id: RomId) -> Option<PathBuf> {
        if let Some(path) = self.loaded_roms.read().unwrap().get(&id) {
            match path {
                LoadedRomLocation::Internal => {
                    return Some(
                        self.environment
                            .read()
                            .unwrap()
                            .rom_store_directory
                            .join(id.to_string()),
                    );
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
