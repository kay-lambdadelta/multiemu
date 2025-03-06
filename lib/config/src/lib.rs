use crate::graphics::GraphicsSettings;
use crate::input::{DEFAULT_HOTKEYS, Hotkey};
use audio::AudioSettings;
use indexmap::IndexMap;
use multiemu_input::{Input, VirtualGamepadName};
use multiemu_rom::system::GameSystem;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use serde_with::serde_as;
use std::{collections::BTreeSet, sync::LazyLock};
use std::{
    fs::{File, create_dir_all},
    ops::Deref,
    path::PathBuf,
};
use strum::{Display, EnumIter};

pub mod audio;
pub mod graphics;
pub mod input;

#[cfg(platform_desktop)]
/// Base directory for the emulators files
pub static STORAGE_DIRECTORY: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::data_dir().unwrap().join("multiemu"));
#[cfg(platform_3ds)]
/// Base directory for the emulators files
pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("sdmc:/multiemu"));
#[cfg(platform_psp)]
/// Base directory for the emulators files
pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("ms0:/multiemu"));

/// Config location
pub static CONFIG_LOCATION: LazyLock<PathBuf> =
    LazyLock::new(|| STORAGE_DIRECTORY.join("config.ron"));

#[derive(Serialize, Deserialize, Debug, Clone, Copy, EnumIter, Display, PartialEq, Eq, Default)]
/// Processor execution mode, if your platform supports JIT
pub enum ProcessorExecutionMode {
    #[cfg_attr(not(jit), default)]
    /// Interpreted mode, slow
    Interpret,
    #[cfg_attr(jit, default)]
    /// JIT mode, faster but less accurate
    Jit,
}

#[serde_as]
#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug)]
/// Miscillaneous settings that the runtime and machine must obey
pub struct Environment {
    #[serde(default)]
    /// Gamepad configs populated by machines or edited by the user
    pub gamepad_configs: IndexMap<GameSystem, IndexMap<VirtualGamepadName, IndexMap<Input, Input>>>,
    #[serde_inline_default(DEFAULT_HOTKEYS.clone())]
    /// Hotkeys for the application
    pub hotkeys: IndexMap<BTreeSet<Input>, Hotkey>,
    #[serde(default)]
    /// Graphics settings
    pub graphics_setting: GraphicsSettings,
    #[serde(default)]
    /// Audio settings
    pub audio_settings: AudioSettings,
    #[serde(default)]
    /// Processor execution mode
    pub processor_execution_mode: ProcessorExecutionMode,
    #[serde_inline_default(STORAGE_DIRECTORY.clone())]
    /// The folder that the gui will show initially
    pub file_browser_home: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("log"))]
    /// Location where logs will be written
    pub log_location: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("database.redb"))]
    /// [redb] database location
    pub database_file: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("saves"))]
    /// Directory where saves will be stored
    pub save_directory: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("snapshot"))]
    /// Directory where snapshots will be stored
    pub snapshot_directory: PathBuf,
    #[serde_inline_default(STORAGE_DIRECTORY.join("roms"))]
    /// Directory where emulator will store imported roms
    pub roms_directory: PathBuf,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            gamepad_configs: Default::default(),
            hotkeys: DEFAULT_HOTKEYS.clone(),
            graphics_setting: GraphicsSettings::default(),
            audio_settings: AudioSettings::default(),
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
    /// Saves the config to the disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        create_dir_all(STORAGE_DIRECTORY.deref())?;
        let config_file = File::create(CONFIG_LOCATION.deref())?;
        ron::ser::to_writer_pretty(
            config_file,
            self,
            PrettyConfig::default()
                .new_line("\n".to_owned())
                .struct_names(false),
        )?;

        Ok(())
    }

    /// Loads the config from the disk
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if !CONFIG_LOCATION.exists() {
            Self::default().save()?;
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
