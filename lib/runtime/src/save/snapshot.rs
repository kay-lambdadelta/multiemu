use crate::{
    component::{ComponentPath, ComponentVersion},
    save::MAGIC,
};
use multiemu_rom::RomId;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub type SlotIndex = u16;

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentSnapshotInfo {
    pub version: ComponentVersion,
    pub data_location: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotHeader {
    pub component_version: ComponentVersion,
    pub magic: [u8; 8],
    pub components: HashMap<ComponentPath, ComponentSnapshotInfo>,
}

#[derive(Debug)]
pub struct Snapshot {
    pub save: Mutex<File>,
    pub version: ComponentVersion,
    pub header: SnapshotHeader,
    pub my_path: ComponentPath,
}

#[derive(Debug)]
pub struct SnapshotManager {
    snapshots: scc::HashMap<(RomId, String, SlotIndex), Arc<Snapshot>>,
    save_directory: Option<PathBuf>,
}

impl SnapshotManager {
    pub fn new(save_directory: Option<PathBuf>) -> Self {
        Self {
            snapshots: Default::default(),
            save_directory,
        }
    }

    pub fn read(
        &self,
        rom_id: RomId,
        component_path: &ComponentPath,
    ) -> Result<Option<Arc<Snapshot>>, Box<dyn std::error::Error>> {
        if self.save_directory.is_none() {
            return Ok(None);
        }

        let mut rom_directory = self
            .save_directory
            .as_ref()
            .unwrap()
            .join(rom_id.to_string());
        rom_directory.extend(component_path.iter());
        let component_file = rom_directory.join("save");

        if !component_file.exists() {
            return Ok(None);
        }

        let mut component_file = File::open(component_file)?;

        let save_file_header: SnapshotHeader =
            bincode::serde::decode_from_std_read(&mut component_file, bincode::config::standard())?;

        // TODO: make custom error type
        if save_file_header.magic != MAGIC {
            return Ok(None);
        }

        todo!()
    }
}
