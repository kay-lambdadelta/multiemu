use super::ExportStyle;
use multiemu_config::Environment;
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use redb::ReadableTable;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub fn rom_export(
    path: PathBuf,
    symlink: bool,
    style: ExportStyle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment = Environment::load()?;
    let rom_manager = Arc::new(
        RomManager::new(
            Some(&environment.database_file),
            Some(&environment.roms_directory),
        )
        .unwrap(),
    );
    fs::create_dir_all(&environment.roms_directory)?;
    fs::create_dir_all(&path)?;

    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
    let database_table = database_transaction.open_table(ROM_INFORMATION_TABLE)?;

    for database_entry in database_table.iter()? {
        let (rom_id, rom_info) = database_entry?;
        let (rom_id, rom_info) = (rom_id.value(), rom_info.value());

        let rom_path = environment.roms_directory.join(rom_id.to_string());

        if rom_path.is_file() {
            tracing::info!("Rom \"{}\" found to be exported", rom_info.name);
        } else {
            continue;
        }

        let target_rom_path = match style {
            ExportStyle::NoIntro => {
                let system_folder_name = rom_info.system.to_string();
                let system_folder = path.join(system_folder_name);
                fs::create_dir_all(&system_folder)?;

                system_folder.join(&rom_info.name)
            }
            ExportStyle::Native => path.join(rom_id.to_string()),
            ExportStyle::EmulationStation => todo!(),
        };

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

    Ok(())
}
