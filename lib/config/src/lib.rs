use crate::graphics::GraphicsSettings;
use crate::input::{Hotkey, DEFAULT_HOTKEYS};
use indexmap::IndexMap;
use multiemu_input::virtual_gamepad::VirtualGamepadId;
use multiemu_input::Input;
use multiemu_rom::system::GameSystem;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use serde_with::serde_as;
use std::{collections::BTreeSet, sync::LazyLock};
use std::{
    fs::{create_dir_all, File},
    ops::Deref,
    path::PathBuf,
};
use strum::{Display, EnumIter};

pub mod graphics;
pub mod input;

/// The directory where we store our runtime files is runtime specific
#[cfg(platform_desktop)]
pub static STORAGE_DIRECTORY: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::data_dir().unwrap().join("multiemu"));
#[cfg(platform_3ds)]
pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("sdmc:/multiemu"));
#[cfg(platform_psp)]
pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("ms0:/multiemu"));

pub static CONFIG_LOCATION: LazyLock<PathBuf> =
    LazyLock::new(|| STORAGE_DIRECTORY.join("config.ron"));

#[derive(Serialize, Deserialize, Debug, Clone, Copy, EnumIter, Display, PartialEq, Eq, Default)]
pub enum ProcessorExecutionMode {
    #[cfg_attr(not(jit), default)]
    Interpret,
    #[cfg(jit)]
    #[cfg_attr(jit, default)]
    Jit,
}

#[serde_as]
#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug)]
pub struct Environment {
    #[serde(default)]
    pub gamepad_configs: IndexMap<GameSystem, IndexMap<VirtualGamepadId, IndexMap<Input, Input>>>,
    #[serde_inline_default(DEFAULT_HOTKEYS.clone())]
    pub hotkeys: IndexMap<BTreeSet<Input>, Hotkey>,
    #[serde(default)]
    pub graphics_setting: GraphicsSettings,
    #[serde(default)]
    pub processor_execution_mode: ProcessorExecutionMode,
    #[serde_inline_default(STORAGE_DIRECTORY.clone())]
    pub file_browser_home: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("log"))]
    pub log_location: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("database.redb"))]
    pub database_file: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("saves"))]
    pub save_directory: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("snapshot"))]
    pub snapshot_directory: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("roms"))]
    pub roms_directory: PathBuf,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            gamepad_configs: Default::default(),
            hotkeys: DEFAULT_HOTKEYS.clone(),
            graphics_setting: GraphicsSettings::default(),
            file_browser_home: STORAGE_DIRECTORY.clone(),
            log_location: STORAGE_DIRECTORY.join("log"),
            database_file: STORAGE_DIRECTORY.join("database.redb"),
            save_directory: STORAGE_DIRECTORY.join("saves"),
            snapshot_directory: STORAGE_DIRECTORY.join("snapshot"),
            roms_directory: STORAGE_DIRECTORY.join("roms"),
            processor_execution_mode: ProcessorExecutionMode::default(),
        }
    }
}

impl Environment {
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        create_dir_all(STORAGE_DIRECTORY.deref())?;
        let config_file = File::create(CONFIG_LOCATION.deref())?;
        ron::ser::to_writer_pretty(config_file, self, PrettyConfig::default())?;

        Ok(())
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if !CONFIG_LOCATION.exists() {
            Self::default().save()?
        }

        let config_file = File::open(CONFIG_LOCATION.deref())?;
        let config = ron::de::from_reader(config_file)?;

        Ok(config)
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        let _ = self.save();
    }
}
