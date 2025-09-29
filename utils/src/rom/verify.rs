use multiemu_config::Environment;
use multiemu_rom::{RomId, RomMetadata};
use std::{
    collections::HashSet,
    fs::{self, File},
    sync::Arc,
};

pub fn rom_verify(
    environment: Environment,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rom_manager = Arc::new(
        RomMetadata::new(
            Some(environment.database_location.0.clone()),
            Some(environment.rom_store_directory.0.clone()),
        )
        .unwrap(),
    );

    fs::create_dir_all(&environment.rom_store_directory.0)?;

    let mut bad_roms = HashSet::new();

    let loaded_roms_guard = rom_manager.loaded_roms.read().unwrap();

    for rom_id in loaded_roms_guard.keys() {
        let rom_path = rom_manager.get_rom_path(*rom_id).unwrap();
        let mut rom_file = File::open(&rom_path).unwrap();
        let calculated_rom_id = RomId::calculate_id(&mut rom_file).unwrap();

        if calculated_rom_id != *rom_id {
            bad_roms.insert((rom_path, rom_id, calculated_rom_id));
        }
    }

    for (rom_path, fake_rom_id, actual_rom_id) in bad_roms {
        tracing::error!(
            "ROM ID mismatch: {} (expected: {}, actual: {})",
            rom_path.display(),
            fake_rom_id,
            actual_rom_id
        );
    }

    Ok(())
}
