use crate::{
    component::{ComponentPath, ComponentVersion},
    machine::registry::ComponentRegistry,
    program::RomId,
};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, create_dir_all, remove_dir_all},
    io::Read,
    path::PathBuf,
};

pub const SAVE_METADATA_FILE_NAME: &str = "metadata.ron";

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub components: HashMap<ComponentPath, ComponentSaveInfo>,
    /// compression always implies zlib compression
    pub compressed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentSaveInfo {
    pub version: ComponentVersion,
}

#[derive(Debug)]
pub struct SaveManager {
    save_directory: Option<PathBuf>,
}

impl SaveManager {
    pub fn new(save_directory: Option<PathBuf>) -> Self {
        Self { save_directory }
    }

    pub fn get(
        &self,
        rom_id: RomId,
        rom_name: &str,
        component_path: ComponentPath,
    ) -> Result<Option<(Box<dyn Read>, ComponentVersion)>, Box<dyn std::error::Error>> {
        let save_directory = match &self.save_directory {
            Some(save_directory) => save_directory,
            None => return Ok(None),
        };

        let save_directory = save_directory.join(rom_id.to_string()).join(rom_name);

        let metadata_path = save_directory.join(SAVE_METADATA_FILE_NAME);
        if !metadata_path.exists() {
            return Ok(None);
        }

        let metadata_file = File::open(&metadata_path)?;
        let metadata: SaveMetadata = ron::de::from_reader(metadata_file)?;

        let component_info = match metadata.components.get(&component_path) {
            Some(info) => info,
            None => return Ok(None),
        };

        let mut save_file_path = save_directory.clone();
        save_file_path.extend(component_path.iter());
        save_file_path.set_extension("bin");

        if !save_file_path.exists() {
            return Ok(None);
        }

        let file = File::open(save_file_path)?;

        if metadata.compressed {
            Ok(Some((
                Box::new(ZlibDecoder::new(file)),
                component_info.version,
            )))
        } else {
            Ok(Some((Box::new(file), component_info.version)))
        }
    }

    pub fn write(
        &self,
        rom_id: RomId,
        rom_name: &str,
        registry: &ComponentRegistry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let save_directory = match &self.save_directory {
            Some(save_directory) => save_directory,
            None => return Ok(()),
        };

        let save_directory = save_directory.join(rom_id.to_string()).join(rom_name);
        let _ = remove_dir_all(&save_directory);

        let mut component_metadata = HashMap::default();

        registry.interact_all(|path, component| {
            let version = component.save_version();

            if let Some(version) = version {
                component_metadata.insert(path.clone(), ComponentSaveInfo { version });
            }
        });

        if component_metadata.is_empty() {
            // Don't write anything if its empty
            return Ok(());
        }

        registry.interact_all(|path, component| {
            // Only write the ones that declared versions
            if component_metadata.contains_key(path) {
                let mut save_file_path = save_directory.clone();
                save_file_path.extend(path.iter());
                save_file_path.set_extension("bin");
                let _ = create_dir_all(save_file_path.parent().unwrap());

                let save_file =
                    ZlibEncoder::new(File::create(save_file_path).unwrap(), Compression::best());

                component.store_save(Box::new(save_file)).unwrap();
            }
        });
        let save_metadata_file = File::create(save_directory.join(SAVE_METADATA_FILE_NAME))?;

        ron::Options::default().to_io_writer_pretty(
            save_metadata_file,
            &SaveMetadata {
                components: component_metadata,
                compressed: true,
            },
            PrettyConfig::default(),
        )?;

        Ok(())
    }
}
