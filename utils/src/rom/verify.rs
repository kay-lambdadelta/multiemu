use std::fs::{self, File};

use fluxemu_frontend::environment::Environment;
use fluxemu_runtime::program::{ProgramManager, RomId};
use rayon::iter::{ParallelBridge, ParallelIterator};

pub fn rom_verify(
    environment: Environment,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let program_manager = ProgramManager::new(
        &environment.database_location,
        &environment.rom_store_directory,
    )
    .unwrap();

    fs::create_dir_all(&environment.rom_store_directory)?;

    program_manager
        .iter_roms()
        .par_bridge()
        .for_each(|(rom_id, _)| {
            let rom_path = program_manager.get_rom_path(rom_id).unwrap();
            let mut rom_file = File::open(&rom_path).unwrap();
            let calculated_rom_id = RomId::calculate_id(&mut rom_file).unwrap();

            if calculated_rom_id != rom_id {
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
