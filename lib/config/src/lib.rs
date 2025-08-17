use crate::graphics::GraphicsSettings;
use audio::AudioSettings;
use multiemu_input::{
    Input, VirtualGamepadName,
    hotkey::{DEFAULT_HOTKEYS, Hotkey},
};
use multiemu_rom::System;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use serde_with::serde_as;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::PathBuf,
    sync::LazyLock,
};
use strum::{Display, EnumIter};

pub mod audio;
pub mod graphics;

cfg_if::cfg_if! {
    if #[cfg(miri)] {
        pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(PathBuf::default);
    } else if #[cfg(target_os = "espidf")] {
        pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/multiemu"));
    } else if #[cfg(target_os = "horizon")] {
        pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("sdmc:/multiemu"));
    } else if #[cfg(target_os = "psp")] {
        pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("ms0:/multiemu"));
    } else if #[cfg(any(target_family = "unix", target_os = "windows"))] {
        pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| dirs::data_dir().unwrap().join("multiemu"));
    } else {
        compile_error!("Unsupported target");
    }
}

/// Config location
pub static ENVIRONMENT_LOCATION: LazyLock<PathBuf> =
    LazyLock::new(|| STORAGE_DIRECTORY.join("config.ron"));

#[derive(Serialize, Deserialize, Debug, Clone, Copy, EnumIter, Display, PartialEq, Eq, Default)]
/// Processor execution mode, if your platform supports JIT
pub enum ProcessorExecutionMode {
    #[default]
    Interpret,
}

#[serde_as]
#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug, Default)]
/// Miscillaneous settings that the runtime and machine must obey
pub struct Environment {
    #[serde(default)]
    /// Gamepad configs populated by machines or edited by the user
    pub gamepad_configs: BTreeMap<System, BTreeMap<VirtualGamepadName, BTreeMap<Input, Input>>>,
    #[serde_inline_default(DEFAULT_HOTKEYS.clone())]
    /// Hotkeys for the application
    ///
    /// Because of this using an array of enums for a key this config should be serialized as ron
    pub hotkeys: BTreeMap<BTreeSet<Input>, Hotkey>,
    #[serde(default)]
    /// Graphics settings
    pub graphics_setting: GraphicsSettings,
    #[serde(default)]
    /// Audio settings
    pub audio_settings: AudioSettings,
    #[serde(default)]
    /// Processor execution mode
    pub processor_execution_mode: ProcessorExecutionMode,
    #[serde(default)]
    /// The folder that the gui will show initially
    pub file_browser_home_directory: FileBrowserHomeDirectory,
    #[serde(default)]
    /// Location where logs will be written
    pub log_location: LogLocation,
    #[serde(default)]
    /// [redb] database location
    pub database_location: DatabaseLocation,
    #[serde(default)]
    /// Directory where saves will be stored
    pub save_directory: SaveDirectory,
    #[serde(default)]
    /// Directory where snapshots will be stored
    pub snapshot_directory: SnapshotDirectory,
    #[serde(default)]
    /// Directory where emulator will store imported roms
    pub rom_store_directory: RomStoreDirectory,
}

impl Environment {
    pub fn save(&self, writer: impl Write) -> Result<(), ron::Error> {
        ron::Options::default().to_io_writer_pretty(
            writer,
            self,
            PrettyConfig::new().struct_names(false),
        )
    }

    pub fn load(reader: impl Read) -> Result<Self, ron::Error> {
        Ok(ron::de::from_reader(reader)?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileBrowserHomeDirectory(pub PathBuf);

impl Default for FileBrowserHomeDirectory {
    fn default() -> Self {
        Self(STORAGE_DIRECTORY.clone())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogLocation(pub PathBuf);

impl Default for LogLocation {
    fn default() -> Self {
        Self(STORAGE_DIRECTORY.join("log"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseLocation(pub PathBuf);

impl Default for DatabaseLocation {
    fn default() -> Self {
        Self(STORAGE_DIRECTORY.join("database.redb"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SnapshotDirectory(pub PathBuf);

impl Default for SnapshotDirectory {
    fn default() -> Self {
        Self(STORAGE_DIRECTORY.join("snapshots"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SaveDirectory(pub PathBuf);

impl Default for SaveDirectory {
    fn default() -> Self {
        Self(STORAGE_DIRECTORY.join("saves"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RomStoreDirectory(pub PathBuf);

impl Default for RomStoreDirectory {
    fn default() -> Self {
        Self(STORAGE_DIRECTORY.join("roms"))
    }
}
