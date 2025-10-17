use multiemu_runtime::{
    environment::Environment,
    program::{ProgramMetadata, RomId},
};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::{
    fs::{self, File},
    sync::{Arc, RwLock},
};

pub fn rom_verify(
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment_guard = environment.read().unwrap();
    let program_manager = ProgramMetadata::new(environment.clone())?;

    fs::create_dir_all(&environment_guard.rom_store_directory)?;

    let loaded_roms_guard = program_manager.loaded_roms.read().unwrap();

    loaded_roms_guard.par_iter().for_each(|(rom_id, _)| {
        let rom_path = program_manager.get_rom_path(*rom_id).unwrap();
        let mut rom_file = File::open(&rom_path).unwrap();
        let calculated_rom_id = RomId::calculate_id(&mut rom_file).unwrap();

        if calculated_rom_id != *rom_id {
            tracing::error!(
                "ROM ID mismatch: {} (expected: {}, actual: {})",
                rom_path.display(),
                calculated_rom_id,
                rom_id
            );
        }
    });

    Ok(())
}
