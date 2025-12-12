use std::{
    collections::HashMap,
    fs::{File, create_dir_all, remove_dir_all},
    io::{BufReader, Read},
    path::PathBuf,
};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::{
    component::ComponentVersion, machine::registry::ComponentRegistry, path::MultiemuPath,
    program::RomId,
};

pub const SNAPSHOT_METADATA_FILE_NAME: &str = "metadata.ron";

pub type SnapshotSlot = u16;

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub components: HashMap<MultiemuPath, ComponentSnapshotInfo>,
    /// compression always implies zlib compression
    pub compressed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentSnapshotInfo {
    pub version: ComponentVersion,
}

#[derive(Debug)]
pub struct SnapshotManager {
    snapshot_directory: Option<PathBuf>,
}

impl SnapshotManager {
    pub fn new(snapshot_directory: Option<PathBuf>) -> Self {
        Self { snapshot_directory }
    }

    pub fn read(
        &self,
        rom_id: RomId,
        rom_name: &str,
        slot: SnapshotSlot,
        registry: &ComponentRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let snapshot_directory = match &self.snapshot_directory {
            Some(snapshot_directory) => snapshot_directory,
            None => return Ok(()),
        };

        let snapshot_directory = snapshot_directory
            .join(rom_id.to_string())
            .join(rom_name)
            .join(slot.to_string());

        if !snapshot_directory.exists() {
            return Ok(());
        }

        let metadata_path = snapshot_directory.join(SNAPSHOT_METADATA_FILE_NAME);
        let metadata: SnapshotMetadata = ron::de::from_reader(File::open(metadata_path)?)?;

        registry.interact_all_mut(|path, component| {
            let component_info = match metadata.components.get(path) {
                Some(info) => info,
                None => return,
            };

            let mut snapshot_file_path = snapshot_directory.clone();
            snapshot_file_path.extend(path.iter());
            snapshot_file_path.set_extension("bin");

            let file =
                BufReader::new(File::open(snapshot_file_path).expect("Missing snapshot file"));

            let snapshot = if metadata.compressed {
                Box::new(ZlibDecoder::new(file)) as Box<dyn Read>
            } else {
                Box::new(file) as Box<dyn Read>
            };

            component
                .load_snapshot(component_info.version, snapshot)
                .unwrap();
        });

        Ok(())
    }

    pub fn write(
        &self,
        rom_id: RomId,
        rom_name: &str,
        slot: SnapshotSlot,
        registry: &ComponentRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let snapshot_directory = match &self.snapshot_directory {
            Some(snapshot_directory) => snapshot_directory,
            None => return Ok(()),
        };

        let snapshot_directory = snapshot_directory
            .join(rom_id.to_string())
            .join(rom_name)
            .join(slot.to_string());
        let _ = remove_dir_all(&snapshot_directory);

        let mut component_metadata = HashMap::default();

        registry.interact_all(|path, component| {
            let version = component.snapshot_version();

            if let Some(version) = version {
                component_metadata.insert(path.clone(), ComponentSnapshotInfo { version });
            }
        });

        if component_metadata.is_empty() {
            // Don't write anything if its empty
            return Ok(());
        }

        registry.interact_all(|path, component| {
            // Only write the ones that declared versions
            if component_metadata.contains_key(path) {
                let mut snapshot_file_path = snapshot_directory.clone();
                snapshot_file_path.extend(path.iter());
                snapshot_file_path.set_extension("bin");
                let _ = create_dir_all(snapshot_file_path.parent().unwrap());

                let snapshot_file = ZlibEncoder::new(
                    File::create(snapshot_file_path).unwrap(),
                    Compression::best(),
                );

                component.store_snapshot(Box::new(snapshot_file)).unwrap();
            }
        });
        let snapshot_metadata_file =
            File::create(snapshot_directory.join(SNAPSHOT_METADATA_FILE_NAME))?;

        ron::Options::default().to_io_writer_pretty(
            snapshot_metadata_file,
            &SnapshotMetadata {
                components: component_metadata,
                compressed: true,
            },
            PrettyConfig::default(),
        )?;

        Ok(())
    }
}
