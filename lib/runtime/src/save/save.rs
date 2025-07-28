use crate::{
    component::{ComponentPath, ComponentVersion},
    save::MAGIC,
};
use multiemu_rom::RomId;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, create_dir_all},
    io::{BufReader, Read, Seek, SeekFrom},
    ops::RangeInclusive,
    path::PathBuf,
    sync::Mutex,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentSaveInfo {
    pub version: ComponentVersion,
    pub data_location: RangeInclusive<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveHeader {
    pub magic: [u8; 8],
    pub components: HashMap<ComponentPath, ComponentSaveInfo>,
}

#[derive(Debug)]
pub struct Save {
    save: Mutex<BufReader<File>>,
    header: SaveHeader,
    path: ComponentPath,
    seek_position: u64,
}

impl Save {
    pub fn version(&self) -> ComponentVersion {
        self.header.components[&self.path].version
    }
}

impl Read for Save {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut file = self.save.lock().unwrap();
        let entry = self.header.components.get(&self.path).unwrap();
        let start = *entry.data_location.start();
        let end = *entry.data_location.end();

        let max_len = end.saturating_sub(start + self.seek_position) + 1;
        if max_len == 0 {
            return Ok(0);
        }

        let len = buf.len().min(max_len as usize);
        file.seek(SeekFrom::Start(start + self.seek_position))?;
        let read = file.read(&mut buf[..len])?;
        self.seek_position += read as u64;
        Ok(read)
    }
}

impl Seek for Save {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let entry = self.header.components.get(&self.path).unwrap();
        let start = *entry.data_location.start();
        let end = *entry.data_location.end();
        let range_len = end.saturating_sub(start) + 1;

        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                let base = range_len as i64 + offset;
                if base < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Negative seek",
                    ));
                }
                base as u64
            }
            SeekFrom::Current(offset) => {
                let base = self.seek_position as i64 + offset;
                if base < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Negative seek",
                    ));
                }
                base as u64
            }
        };

        if new_pos > range_len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Seek out of range",
            ));
        }

        self.seek_position = new_pos;
        Ok(new_pos)
    }
}

#[derive(Debug)]
pub struct SaveManager {
    save_directory: Option<PathBuf>,
}

impl SaveManager {
    pub fn new(save_directory: Option<PathBuf>) -> Self {
        Self { save_directory }
    }

    pub fn read(
        &self,
        rom_id: RomId,
        rom_name: &str,
        component_path: ComponentPath,
    ) -> Result<Option<Save>, Box<dyn std::error::Error>> {
        if self.save_directory.is_none() {
            return Ok(None);
        }

        let rom_directory = self
            .save_directory
            .as_ref()
            .unwrap()
            .join(rom_id.to_string());
        create_dir_all(&rom_directory).unwrap();
        let save_file = rom_directory.join(rom_name);

        if !save_file.exists() {
            return Ok(None);
        }

        let mut component_file = File::open(save_file)?;

        let save_file_header: SaveHeader =
            bincode::serde::decode_from_std_read(&mut component_file, bincode::config::standard())?;

        // TODO: make custom error type
        if save_file_header.magic != MAGIC {
            return Ok(None);
        }

        if !save_file_header.components.contains_key(&component_path) {
            return Ok(None);
        }

        Ok(Some(Save {
            save: Mutex::new(BufReader::new(component_file)),
            header: save_file_header,
            path: component_path,
            seek_position: 0,
        }))
    }
}
