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
/// Individual component snapshot
pub struct ComponentSnapshot {
    /// Version of the component
    pub component_version: ComponentVersion,
    /// Data of the component
    pub component_data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
/// On disk format for snapshots
pub struct SnapshotFile {
    /// Components
    pub components: HashMap<ComponentName, ComponentSnapshot>,
}

#[derive(Debug)]
/// Manager helper for snapshots
pub struct SnapshotManager {
    snapshot_files: scc::HashCache<(RomId, u8), Arc<SnapshotFile>>,
    snapshot_directory: Option<PathBuf>,
}

impl SnapshotManager {
    /// Pass in [None] for tests
    pub fn new(snapshot_directory: Option<PathBuf>) -> Self {
        Self {
            snapshot_files: scc::HashCache::with_capacity(0, 4),
            snapshot_directory,
        }
    }

    /// Retrieve a snapshot if it exists
    pub fn get(
        &self,
        slot: u8,
        rom_id: RomId,
    ) -> Result<Option<Arc<SnapshotFile>>, Box<dyn std::error::Error>> {
        let entry = match self.snapshot_files.entry((rom_id, slot)) {
            scc::hash_cache::Entry::Occupied(occupied_entry) => occupied_entry,
            scc::hash_cache::Entry::Vacant(vacant_entry) => {
                if self.snapshot_directory.is_none() {
                    return Ok(None);
                }

                let path = self
                    .snapshot_directory
                    .as_ref()
                    .unwrap()
                    .join(rom_id.to_string());
                create_dir_all(&path)?;
                let path = path.join(slot.to_string());

                if !path.exists() {
                    return Ok(None);
                }

                let mut snapshot_file = File::open(path)?;
                let snapshot_file = Arc::new(bincode::serde::decode_from_std_read(
                    &mut snapshot_file,
                    bincode::config::standard(),
                )?);

                let (_, entry) = vacant_entry.put_entry(snapshot_file);

                entry
            }
        };

        Ok(Some(entry.clone()))
    }

    /// Add a snapshot to the store
    pub fn insert(
        &mut self,
        slot: u8,
        rom_id: RomId,
        save: Arc<SnapshotFile>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.snapshot_files.put((rom_id, slot), save.clone());

        let path = self
            .snapshot_directory
            .as_ref()
            .unwrap()
            .join(rom_id.to_string());
        create_dir_all(&path)?;
        let path = path.join(slot.to_string());

        let mut save_file = File::create(path)?;
        bincode::serde::encode_into_std_write(
            save.deref(),
            &mut save_file,
            bincode::config::standard(),
        )?;

        Ok(())
    }
}
