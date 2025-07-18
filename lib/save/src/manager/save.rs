use crate::{ComponentName, ComponentVersion};
use multiemu_rom::RomId;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, create_dir_all},
    ops::Deref,
    path::PathBuf,
    sync::Arc,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentSave {
    pub component_version: ComponentVersion,
    pub component_data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveFile {
    pub components: HashMap<ComponentName, ComponentSave>,
}

#[derive(Debug)]
pub struct SaveManager {
    save_files: scc::HashCache<RomId, Arc<SaveFile>>,
    save_directory: Option<PathBuf>,
}

impl SaveManager {
    pub fn new(save_directory: Option<PathBuf>) -> Self {
        Self {
            save_files: scc::HashCache::with_capacity(0, 4),
            save_directory,
        }
    }

    pub fn get(&self, rom_id: RomId) -> Result<Option<Arc<SaveFile>>, Box<dyn std::error::Error>> {
        let entry = match self.save_files.entry(rom_id) {
            scc::hash_cache::Entry::Occupied(occupied_entry) => occupied_entry,
            scc::hash_cache::Entry::Vacant(vacant_entry) => {
                let path = self.save_directory.as_ref().unwrap();
                create_dir_all(&path)?;
                let path = path.join(rom_id.to_string());

                if !path.exists() {
                    return Ok(None);
                }

                let mut save_file = File::open(path)?;
                let save_file = Arc::new(bincode::serde::decode_from_std_read(
                    &mut save_file,
                    bincode::config::standard(),
                )?);

                let (_, entry) = vacant_entry.put_entry(save_file);

                entry
            }
        };

        Ok(Some(entry.clone()))
    }

    pub fn insert(
        &mut self,
        rom_id: RomId,
        save: Arc<SaveFile>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.save_files.put(rom_id, save.clone());

        let path = self.save_directory.as_ref().unwrap();
        create_dir_all(&path)?;
        let path = path.join(rom_id.to_string());

        let mut save_file = File::create(path)?;
        bincode::serde::encode_into_std_write(
            save.deref(),
            &mut save_file,
            bincode::config::standard(),
        )?;

        Ok(())
    }
}
