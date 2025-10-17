use multiemu_runtime::{
    environment::Environment,
    program::{HASH_ALIAS_TABLE, ProgramId, ProgramMetadata, RomId},
};
use redb::{ReadOnlyMultimapTable, ReadableDatabase};
use scc::{HashCache, hash_cache::OccupiedEntry};
use std::{
    collections::VecDeque,
    error::Error,
    fmt::Display,
    fs::{self, File},
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, RwLock},
};
use zip::ZipArchive;

/// Cache to try to avoid reading the metadata of the same file multiple times
static ZIP_CACHE: LazyLock<ZipCache> = LazyLock::new(ZipCache::default);

pub struct ZipCache(HashCache<PathBuf, ZipArchive<File>>);

impl Default for ZipCache {
    fn default() -> Self {
        Self(HashCache::with_capacity(
            0,
            rayon::current_num_threads() * 2,
        ))
    }
}

impl ZipCache {
    pub fn get(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<OccupiedEntry<'_, PathBuf, ZipArchive<File>>, Box<dyn Error + Send + Sync>> {
        let path = path.as_ref();

        match self.0.entry_sync(path.to_path_buf()) {
            scc::hash_cache::Entry::Occupied(occupied_entry) => Ok(occupied_entry),
            scc::hash_cache::Entry::Vacant(vacant_entry) => {
                let archive = ZipArchive::new(File::open(path)?)?;
                let (_, entry) = vacant_entry.put_entry(archive);

                Ok(entry)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum ArchiveType {
    Zip,
}

#[derive(Debug, Clone)]
enum SearchEntry {
    File(PathBuf),
    Archive {
        archive_path: PathBuf,
        archive_type: ArchiveType,
        path: PathBuf,
    },
}

impl Display for SearchEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchEntry::File(path) => write!(f, "{}", path.display()),
            SearchEntry::Archive {
                archive_path,
                archive_type: ArchiveType::Zip,
                path,
            } => write!(f, "{}:{}", archive_path.display(), path.display()),
        }
    }
}

pub fn rom_import(
    paths: Vec<PathBuf>,
    symlink: bool,
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment_guard = environment.read().unwrap();

    let program_manager = Arc::new(ProgramMetadata::new(environment.clone()).unwrap());

    fs::create_dir_all(&environment_guard.rom_store_directory)?;

    rayon::scope(|scope| {
        let mut stack = VecDeque::from_iter(paths.into_iter().map(SearchEntry::File));

        while let Some(entry) = stack.pop_front() {
            match entry {
                SearchEntry::File(path) => {
                    if path.is_dir() {
                        tracing::debug!("Found directory \"{}\"", path.display());

                        if let Ok(directory_entries) = fs::read_dir(path) {
                            stack.extend(directory_entries.into_iter().filter_map(|entry| {
                                entry.ok().map(|entry| SearchEntry::File(entry.path()))
                            }));
                        }
                    } else if path.is_file() {
                        // Try looking at the whole file itself

                        {
                            let program_manager = program_manager.clone();
                            let environment = environment.clone();

                            let path = path.clone();
                            scope.spawn(move |_| {
                                if let Err(err) = process_file(
                                    SearchEntry::File(path.clone()),
                                    symlink,
                                    program_manager,
                                    environment,
                                ) {
                                    tracing::error!(
                                        "Failed to process file \"{}\": {}",
                                        path.display(),
                                        err
                                    );
                                }
                            });
                        }

                        // Try to parse as zip file
                        if let Ok(mut archive) = ZIP_CACHE.get(&path) {
                            tracing::debug!(
                                "File \"{}\" is a zip archive with {} entries",
                                path.display(),
                                archive.len()
                            );

                            for index in 0..archive.len() {
                                if let Ok(file) = archive.by_index(index) {
                                    let Some(internal_path) = file.enclosed_name() else {
                                        continue;
                                    };

                                    stack.push_back(SearchEntry::Archive {
                                        archive_path: path.clone(),
                                        archive_type: ArchiveType::Zip,
                                        path: internal_path,
                                    });
                                } else {
                                    tracing::error!(
                                        "Failed to read entry {} in archive \"{}\"",
                                        index,
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                }
                SearchEntry::Archive {
                    archive_path,
                    archive_type,
                    path,
                } => {
                    let program_manager = program_manager.clone();
                    let environment = environment.clone();

                    scope.spawn(move |_| {
                        let _ = process_file(
                            SearchEntry::Archive {
                                archive_path,
                                archive_type,
                                path,
                            },
                            symlink,
                            program_manager,
                            environment,
                        );
                    });
                }
            }
        }
    });

    Ok(())
}

fn process_file(
    entry: SearchEntry,
    symlink: bool,
    program_manager: Arc<ProgramMetadata>,
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let database_transaction = program_manager.database.begin_read()?;
    let hash_alias_table = database_transaction.open_multimap_table(HASH_ALIAS_TABLE)?;

    match entry.clone() {
        SearchEntry::File(path) => {
            let file = File::open(&path)?;
            let (rom_id, converted_reader) = convert_and_fetch_id(path, file)?;

            process_file_internal(
                entry,
                symlink,
                environment,
                hash_alias_table,
                rom_id,
                converted_reader,
            )?;
        }
        SearchEntry::Archive {
            archive_path,
            archive_type: ArchiveType::Zip,
            path,
        } => {
            let rom_id = {
                let mut archive = ZIP_CACHE.get(&archive_path)?;

                let index = archive.index_for_path(&path).ok_or_else(|| {
                    format!("Could not find entry \"{}\" in archive", path.display())
                })?;

                let mut file = archive.by_index(index)?;

                RomId::calculate_id(&mut file)?
            };

            process_file_internal(
                entry,
                symlink,
                environment,
                hash_alias_table,
                rom_id,
                // TODO: Autoconversions for files inside zips
                None::<&[u8]>,
            )?;
        }
    };

    Ok(())
}

fn process_file_internal(
    entry: SearchEntry,
    symlink: bool,
    environment: Arc<RwLock<Environment>>,
    hash_alias_table: ReadOnlyMultimapTable<RomId, ProgramId>,
    rom_id: RomId,
    mut converted_reader: Option<impl Read>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let environment_guard = environment.read().unwrap();

    if let Some(program_id) = hash_alias_table.get(&rom_id)?.next() {
        let program_id = program_id?.value();

        tracing::info!(
            "Found ROM {} that is used in program {}",
            rom_id,
            program_id,
        );

        let internal_rom_path = environment_guard
            .rom_store_directory
            .join(rom_id.to_string());
        let _ = fs::remove_file(&internal_rom_path);

        if let Some(mut converted_reader) = converted_reader.take() {
            let mut internal_rom_file = File::create(&internal_rom_path)?;

            std::io::copy(&mut converted_reader, &mut internal_rom_file)?;
        } else {
            match entry {
                SearchEntry::File(path) => {
                    if symlink {
                        #[cfg(target_family = "unix")]
                        {
                            std::os::unix::fs::symlink(&path, &internal_rom_path)?;
                        }

                        #[cfg(windows)]
                        {
                            std::os::windows::fs::symlink_file(&path, &internal_rom_path)?;
                        }

                        #[cfg(not(any(unix, windows)))]
                        panic!("Unsupported platform for symlinks");
                    } else {
                        fs::copy(&path, &internal_rom_path)?;
                    }
                }
                SearchEntry::Archive {
                    archive_path,
                    archive_type: ArchiveType::Zip,
                    path,
                } => {
                    let mut archive = ZIP_CACHE.get(&archive_path)?;
                    let index = archive.index_for_path(&path).ok_or_else(|| {
                        format!("Could not find entry \"{}\" in archive", path.display())
                    })?;
                    let mut file = archive.by_index(index)?;
                    let mut internal_rom_file = File::create(&internal_rom_path)?;
                    std::io::copy(&mut file, &mut internal_rom_file)?;
                }
            }
        }
    } else {
        tracing::debug!("Could not identify ROM {} at \"{}\"", rom_id, entry);
    };

    Ok(())
}

fn convert_and_fetch_id(
    path: impl AsRef<Path>,
    mut rom: impl Read + Seek + Send + 'static,
) -> Result<(RomId, Option<impl Read>), Box<dyn Error + Send + Sync>> {
    let path = path.as_ref();

    if let Some(file_extention) = path.extension().and_then(|ext| ext.to_str()) {
        match file_extention.to_lowercase().as_str() {
            "rvz" | "wia" => {
                tracing::info!(
                    "Converted detected non-standard format to ISO for \"{}\"",
                    path.display()
                );

                let mut converted_reader = crate::convert::wiigamecube::to_iso(rom)?;
                converted_reader.rewind()?;

                let rom_id = RomId::calculate_id(&mut converted_reader)?;
                converted_reader.rewind()?;

                Ok((rom_id, Some(converted_reader)))
            }
            _ => {
                let rom_id = RomId::calculate_id(&mut rom)?;

                Ok((rom_id, None))
            }
        }
    } else {
        Ok((RomId::calculate_id(&mut rom)?, None))
    }
}
