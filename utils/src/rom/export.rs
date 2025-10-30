use super::ExportStyle;
use multiemu_runtime::{
    environment::Environment,
    program::{Filesystem, PROGRAM_INFORMATION_TABLE, ProgramManager},
};
use redb::{ReadableDatabase, ReadableMultimapTable};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub fn rom_export(
    path: PathBuf,
    symlink: bool,
    style: ExportStyle,
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment_guard = environment.read().unwrap();

    let program_manager = ProgramManager::new(environment.clone()).unwrap();

    fs::create_dir_all(&environment_guard.rom_store_directory)?;
    fs::create_dir_all(&path)?;

    let database_transaction = program_manager.database().begin_read().unwrap();
    let database_table = database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

    for database_entry in database_table.iter()? {
        let (rom_id, rom_infos) = database_entry?;

        for program_info in rom_infos {
            let (program_id, program_info) = (rom_id.value(), program_info?.value());

            let mut roms_to_export: HashMap<PathBuf, HashSet<_>> = HashMap::new();

            match program_info.filesystem() {
                Filesystem::Single { rom_id, file_name } => {
                    let rom_path = environment_guard
                        .rom_store_directory
                        .join(rom_id.to_string());

                    if rom_path.is_file() {
                        tracing::info!("ROM for program {} found to be exported", program_id);
                    } else {
                        continue;
                    }

                    roms_to_export
                        .entry(rom_path)
                        .or_default()
                        .insert(PathBuf::from(file_name));
                }
                Filesystem::Complex(fs) => {
                    for (rom_id, path) in fs
                        .iter()
                        .flat_map(|(rom_id, paths)| paths.iter().map(|path| (*rom_id, path)))
                    {
                        let rom_path = environment_guard
                            .rom_store_directory
                            .join(rom_id.to_string());

                        if rom_path.is_file() {
                            tracing::info!("ROM for program {} found to be exported", program_id);
                        } else {
                            continue;
                        }

                        roms_to_export
                            .entry(rom_path)
                            .or_default()
                            .insert(PathBuf::from_iter(path.split('/')));
                    }
                }
            }

            for (import_path, export_paths) in roms_to_export {
                for export_path in export_paths {
                    let target_rom_path = match style {
                        ExportStyle::NoIntro => {
                            let machine_folder_name = program_id.machine.to_nointro_string();
                            let machine_folder = path.join(machine_folder_name);
                            let game_folder = machine_folder.join(&program_id.name);
                            let final_path = game_folder.join(&export_path);

                            fs::create_dir_all(final_path.parent().unwrap())?;

                            final_path
                        }
                        ExportStyle::Native => todo!(),
                        ExportStyle::EmulationStation => todo!(),
                    };

                    if !target_rom_path.starts_with(&path) {
                        tracing::error!("Export path is outside of the target directory");
                        continue;
                    }

                    tracing::info!("Exporting {:?} to {:?}", import_path, export_path);

                    let _ = fs::remove_file(&target_rom_path);
                    if symlink {
                        #[cfg(target_family = "unix")]
                        std::os::unix::fs::symlink(&import_path, target_rom_path)?;

                        #[cfg(windows)]
                        std::os::windows::fs::symlink_file(&import_path, target_rom_path)?;

                        #[cfg(not(any(unix, windows)))]
                        panic!("Unsupported runtime for symlinking");
                    } else {
                        fs::copy(&import_path, target_rom_path)?;
                    }
                }
            }
        }
    }

    Ok(())
}
