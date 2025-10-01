use super::ExportStyle;
use multiemu::{
    environment::Environment,
    rom::{ROM_INFORMATION_TABLE, RomMetadata},
};
use redb::{ReadableDatabase, ReadableMultimapTable};
use std::{
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

    let rom_manager = Arc::new(RomMetadata::new(environment.clone()).unwrap());

    fs::create_dir_all(&environment_guard.rom_store_directory)?;
    fs::create_dir_all(&path)?;

    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
    let database_table = database_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;

    for database_entry in database_table.iter()? {
        let (rom_id, rom_infos) = database_entry?;

        for rom_info in rom_infos {
            let (rom_id, rom_info) = (rom_id.value(), rom_info?.value());

            let rom_path = environment_guard
                .rom_store_directory
                .join(rom_id.to_string());

            if rom_path.is_file() {
                tracing::info!(
                    "ROM {:?} found to be exported",
                    PathBuf::from_iter(rom_info.path().iter())
                );
            } else {
                continue;
            }

            let target_rom_path = match style {
                ExportStyle::NoIntro => {
                    let system_folder_name = rom_info.system().to_string();
                    let system_folder = path.join(system_folder_name);
                    let game_folder = system_folder.join(rom_info.name());
                    let final_path = game_folder.join(PathBuf::from_iter(rom_info.path().iter()));

                    fs::create_dir_all(final_path.parent().unwrap())?;

                    final_path
                }
                ExportStyle::Native => path.join(rom_id.to_string()),
                ExportStyle::EmulationStation => todo!(),
            };

            if !target_rom_path.starts_with(&path) {
                tracing::error!("Export path is outside of the target directory");
                continue;
            }

            let _ = fs::remove_file(&target_rom_path);
            if symlink {
                #[cfg(unix)]
                std::os::unix::fs::symlink(rom_path, target_rom_path)?;

                #[cfg(windows)]
                std::os::windows::fs::symlink_file(rom_path, target_rom_path)?;

                #[cfg(not(any(unix, windows)))]
                panic!("Unsupported runtime for symlinking");
            } else {
                fs::copy(rom_path, target_rom_path)?;
            }
        }
    }

    Ok(())
}
