use crate::program::{
    MachineId, ProgramId, ProgramInfo, ProgramSpecification, RomId, info::Filesystem,
};
use redb::{
    Database, MultimapTableDefinition, ReadableDatabase, ReadableMultimapTable,
    backends::InMemoryBackend,
};
use rustc_hash::FxBuildHasher;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    fs::{File, create_dir_all},
    io::Seek,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, LazyLock, Weak},
};

/// Program id -> Program info mapping
pub const PROGRAM_INFORMATION_TABLE: MultimapTableDefinition<ProgramId, ProgramInfo> =
    MultimapTableDefinition::new("program_information");
/// Hash -> Program id reverse mapping
pub const HASH_ALIAS_TABLE: MultimapTableDefinition<RomId, ProgramId> =
    MultimapTableDefinition::new("hash_alias");

static DATABASE_CACHE: LazyLock<scc::HashMap<PathBuf, Weak<Database>>> =
    LazyLock::new(Default::default);

/// The ROM manager which contains the database and information about the roms that were loaded
#[derive(Debug)]
pub struct ProgramManager {
    database: Arc<Database>,
    external_roms: scc::HashMap<RomId, PathBuf, FxBuildHasher>,
    rom_store: PathBuf,
}

impl Default for ProgramManager {
    fn default() -> Self {
        let database = Database::builder()
            .create_with_backend(InMemoryBackend::default())
            .unwrap();

        let mut database_transaction = database.begin_write().unwrap();
        database_transaction.set_quick_repair(true);
        database_transaction
            .open_multimap_table(PROGRAM_INFORMATION_TABLE)
            .unwrap();
        database_transaction
            .open_multimap_table(HASH_ALIAS_TABLE)
            .unwrap();
        database_transaction.commit().unwrap();

        Self {
            database: Arc::new(database),
            external_roms: Default::default(),
            rom_store: PathBuf::default(),
        }
    }
}

impl ProgramManager {
    /// Opens and loads the default database
    pub fn new(
        database: impl AsRef<Path>,
        rom_store: impl AsRef<Path>,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let database_path = database.as_ref();
        let rom_store_path = rom_store.as_ref();

        let _ = create_dir_all(database_path.parent().unwrap());
        let _ = create_dir_all(rom_store_path);

        tracing::info!("Loading ROM database at {:?}", database_path);

        let database_entry = DATABASE_CACHE.entry_sync(rom_store_path.to_owned());

        let database = if let scc::hash_map::Entry::Occupied(cached_database) = &database_entry
            && let Some(cached_database) = cached_database.upgrade()
        {
            cached_database
        } else {
            let database = Database::builder().create(database_path).unwrap();

            let mut database_transaction = database.begin_write().unwrap();
            database_transaction.set_quick_repair(true);
            database_transaction
                .open_multimap_table(PROGRAM_INFORMATION_TABLE)
                .unwrap();
            database_transaction
                .open_multimap_table(HASH_ALIAS_TABLE)
                .unwrap();
            database_transaction.commit().unwrap();

            let database = Arc::new(database);
            database_entry.insert_entry(Arc::downgrade(&database));
            database
        };

        Ok(Arc::new(Self {
            database,
            external_roms: scc::HashMap::default(),
            rom_store: rom_store_path.to_path_buf(),
        }))
    }

    /// Opens a ROM, giving a warning or panicking in the case that the requirement is not met
    ///
    /// Components should use this instead of directly opening ROM files
    pub fn open(&self, id: RomId, requirement: RomRequirement) -> Option<File> {
        if let Some(path) = self.get_rom_path(id) {
            if path.is_file() {
                return Some(File::open(path).unwrap());
            }
            tracing::error!("ROM {} is not a file", id);
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

    /// Attempts to identify a program from its paths
    #[tracing::instrument(skip_all)]
    pub fn identify_program_from_paths<'a>(
        &'a self,
        roms: impl IntoIterator<Item = PathBuf> + 'a,
    ) -> Result<Option<ProgramSpecification>, Box<dyn std::error::Error>> {
        let rom_paths = Vec::from_iter(roms);
        let mut rom_files = HashMap::new();

        for path in &rom_paths {
            let file = File::open(path)?;
            rom_files.insert(path, file);
        }

        let mut hashes = Vec::default();

        for (rom_path, rom) in &mut rom_files {
            let sha1 = RomId::calculate_id(rom)?;
            rom.rewind()?;

            self.external_roms.upsert_sync(sha1, (*rom_path).clone());
            hashes.push(sha1);
        }

        if let Some(specification) = self.identify_program(hashes.clone())? {
            return Ok(Some(specification));
        }

        if let Some((path, machine_id)) = rom_paths.into_iter().find_map(|path| {
            // Try to figure out what machine this rom is for
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

    /// Attempts to identify a program from its program ids
    ///
    /// Prefer [`Self::identify_program_from_paths`] when possible, as it will have a higher success rate
    #[tracing::instrument(skip_all)]
    pub fn identify_program(
        &self,
        roms: impl IntoIterator<Item = RomId>,
    ) -> Result<Option<ProgramSpecification>, Box<dyn std::error::Error>> {
        let roms = Vec::from_iter(roms);
        let read_transaction = self.database().begin_read()?;
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
                    && let Some(path) = self.get_rom_path(roms[0])
                {
                    let rom_id = roms[0];

                    let internal_path = self.get_rom_path(rom_id)?;

                    let name = path
                        .with_extension("")
                        .file_name()
                        .map(|string| string.to_string_lossy().to_string())?;

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
        self.external_roms
            .get_sync(&id)
            .map(|entry| entry.clone())
            .or_else(|| {
                let potential_path = self.rom_store.join(id.to_string());

                if potential_path.is_file() {
                    return Some(potential_path);
                }

                None
            })
    }

    /// Imports a arbitary database into the internal database
    pub fn load_database(&self, path: impl AsRef<Path>) -> Result<(), redb::Error> {
        let path = path.as_ref();

        let database = redb::Database::builder().open_read_only(path)?;
        let external_database_transaction = database.begin_read()?;
        let external_database_table =
            external_database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

        for item in external_database_table.iter()? {
            let (rom_id, rom_infos) = item?;

            let internal_database_transaction = self.database().begin_write()?;
            let mut internal_database_table =
                internal_database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

            for rom_info in rom_infos {
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

    pub fn database(&self) -> &Database {
        &self.database
    }

    pub fn iter_roms(&self) -> impl Iterator<Item = (RomId, PathBuf)> {
        let mut already_visited = HashSet::new();
        let mut external_roms = Vec::default();

        self.external_roms.iter_sync(|id, path| {
            external_roms.push((*id, path.clone()));
            already_visited.insert(*id);

            true
        });

        let rom_store_listings = self.rom_store.read_dir();

        external_roms
            .into_iter()
            .chain(
                rom_store_listings
                    .into_iter()
                    .flatten()
                    .filter_map(move |dir_entry| {
                        let dir_entry = dir_entry.ok()?;

                        let file_name = dir_entry.file_name().into_string().ok()?;
                        let rom_id = RomId::from_str(&file_name).ok()?;

                        if already_visited.contains(&rom_id) {
                            None
                        } else {
                            already_visited.insert(rom_id);
                            Some((rom_id, dir_entry.path()))
                        }
                    }),
            )
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
