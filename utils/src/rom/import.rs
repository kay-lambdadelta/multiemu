use multiemu_config::Environment;
use multiemu_rom::{
    id::RomId,
    manager::{ROM_INFORMATION_TABLE, RomManager},
};
use scc::{HashCache, hash_cache::OccupiedEntry};
use std::{
    cell::LazyCell,
    collections::VecDeque,
    error::Error,
    fmt::Display,
    fs::{self, File},
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
};
use zip::ZipArchive;

thread_local! {
    /// Cache to try to avoid reading the metadata of the same file multiple times
    static ZIP_CACHE: LazyCell<ZipCache> = LazyCell::new(ZipCache::default);
}

pub struct ZipCache(HashCache<PathBuf, ZipArchive<File>>);

impl Default for ZipCache {
    fn default() -> Self {
        Self(HashCache::with_capacity(0, 4))
    }
}

impl ZipCache {
    pub fn get(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<OccupiedEntry<PathBuf, ZipArchive<File>>, Box<dyn Error + Send + Sync>> {
        let path = path.as_ref();

        match self.0.entry(path.to_path_buf()) {
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment = Arc::new(Environment::load()?);
    let rom_manager = Arc::new(
        RomManager::new(
            Some(&environment.database_file),
            Some(&environment.roms_directory),
        )
        .unwrap(),
    );
    fs::create_dir_all(&environment.roms_directory)?;

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
                            let rom_manager = rom_manager.clone();
                            let environment = environment.clone();
                            let path = path.clone();
                            scope.spawn(move |_| {
                                if let Err(err) = process_file(
                                    SearchEntry::File(path.clone()),
                                    symlink,
                                    rom_manager,
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

                        ZIP_CACHE.with(|zip_cache| {
                            // Try to parse as zip file
                            if let Ok(mut archive) = zip_cache.get(&path) {
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
                        });
                    }
                }
                SearchEntry::Archive {
                    archive_path,
                    archive_type,
                    path,
                } => {
                    let rom_manager = rom_manager.clone();
                    let environment = environment.clone();

                    scope.spawn(move |_| {
                        let _ = process_file(
                            SearchEntry::Archive {
                                archive_path,
                                archive_type,
                                path,
                            },
                            symlink,
                            rom_manager,
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
    rom_manager: Arc<RomManager>,
    environment: Arc<Environment>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let database_transaction = rom_manager.rom_information.begin_read()?;
    let database_table = database_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;

    let mut converted_reader = None;

    let rom_id: RomId = match entry.clone() {
        SearchEntry::File(path) => {
            let file = File::open(&path)?;
            let (rom_id, new_converted_reader) = convert_and_fetch_id(path, file)?;

            if let Some(new_converted_reader) = new_converted_reader {
                converted_reader = Some(new_converted_reader);
            }

            rom_id
        }
        SearchEntry::Archive {
            archive_path,
            archive_type: ArchiveType::Zip,
            path,
        } => ZIP_CACHE.with(|zip_cache| {
            let mut archive = zip_cache.get(&archive_path)?;
            let index = archive
                .index_for_path(&path)
                .ok_or_else(|| format!("Could not find entry \"{}\" in archive", path.display()))?;
            let file = archive.by_index(index)?;

            Ok::<_, Box<dyn Error + Send + Sync>>(RomId::from_read(file))
        })?,
    };

    // Just fetch the first one
    if let Some(rom_info) = database_table.get(&rom_id)?.next() {
        let rom_info = rom_info?.value();

        tracing::info!(
            "Found ROM {} with name \"{}\" in file \"{}\"",
            rom_id,
            rom_info.file_name,
            entry
        );

        let internal_rom_path = environment.roms_directory.join(rom_id.to_string());
        let _ = fs::remove_file(&internal_rom_path);

        if let Some(mut converted_reader) = converted_reader.take() {
            let mut internal_rom_file = File::create(&internal_rom_path)?;

            std::io::copy(&mut converted_reader, &mut internal_rom_file)?;
        } else {
            match entry {
                SearchEntry::File(path) => {
                    if symlink {
                        #[cfg(unix)]
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
                    ZIP_CACHE.with(|zip_cache| {
                        let mut archive = zip_cache.get(&archive_path)?;
                        let index = archive.index_for_path(&path).ok_or_else(|| {
                            format!("Could not find entry \"{}\" in archive", path.display())
                        })?;
                        let mut file = archive.by_index(index)?;

                        let mut internal_rom_file = File::create(&internal_rom_path)?;
                        std::io::copy(&mut file, &mut internal_rom_file)?;

                        Ok::<_, Box<dyn Error + Send + Sync>>(())
                    })?;
                }
            }
        }
    } else {
        tracing::debug!("Could not identify ROM {} at \"{}\"", rom_id, entry);
    }

    Ok(())
}

fn convert_and_fetch_id(
    path: impl AsRef<Path>,
    rom_file: File,
) -> Result<(RomId, Option<impl Read>), Box<dyn Error + Send + Sync>> {
    let path = path.as_ref();

    if let Some(file_extention) = path.extension().and_then(|ext| ext.to_str()) {
        match file_extention.to_lowercase().as_str() {
            "rvz" | "wia" | "wbfs" | "ciso" | "nfs" | "gcz" => {
                tracing::info!(
                    "Converted detected non-standard format to ISO for \"{}\"",
                    path.display()
                );

                let mut converted_reader = crate::convert::wiigamecube::to_iso(rom_file)?;
                converted_reader.rewind()?;

                let rom_id = RomId::from_read(&mut converted_reader);
                converted_reader.rewind()?;

                Ok((rom_id, Some(converted_reader)))
            }
            _ => {
                let rom_id = RomId::from_read(rom_file);

                Ok((rom_id, None))
            }
        }
    } else {
        Ok((RomId::from_read(rom_file), None))
    }
}
