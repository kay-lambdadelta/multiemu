use crate::{
    component::ResourcePath,
    environment::graphics::GraphicsSettings,
    input::{
        Input,
        hotkey::{DEFAULT_HOTKEYS, Hotkey},
    },
    program::MachineId,
};
use audio::AudioSettings;
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

/// Audio related config types
pub mod audio;
/// Graphics related config types
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
        /// Directory that multiemu will use as a "home" folder
        pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| dirs::data_dir().unwrap().join("multiemu"));
    } else {
        compile_error!("Unsupported target");
    }
}

/// Config location
pub static ENVIRONMENT_LOCATION: LazyLock<PathBuf> =
    LazyLock::new(|| STORAGE_DIRECTORY.join("config.ron"));

#[serde_as]
#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug)]
/// Miscellaneous settings that the runtime and machine must obey
///
/// The canonical on-disk representation for this config is RON but any type will do
pub struct Environment {
    #[serde(default)]
    /// Gamepad configs populated by machines or edited by the user
    pub gamepad_configs: BTreeMap<MachineId, BTreeMap<ResourcePath, BTreeMap<Input, Input>>>,
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
    #[serde_inline_default(Environment::default().file_browser_home_directory)]
    /// The folder that the gui will show initially
    pub file_browser_home_directory: PathBuf,
    #[serde_inline_default(Environment::default().log_location)]
    /// Location where logs will be written
    pub log_location: PathBuf,
    #[serde_inline_default(Environment::default().database_location)]
    /// [redb] database location
    pub database_location: PathBuf,
    #[serde_inline_default(Environment::default().save_directory)]
    /// Directory where saves will be stored
    pub save_directory: PathBuf,
    #[serde_inline_default(Environment::default().snapshot_directory)]
    /// Directory where snapshots will be stored
    pub snapshot_directory: PathBuf,
    #[serde_inline_default(Environment::default().rom_store_directory)]
    /// Directory where emulator will store imported roms
    pub rom_store_directory: PathBuf,
}

impl Environment {
    /// Store the config from a file
    pub fn save(&self, writer: impl Write) -> Result<(), ron::Error> {
        ron::Options::default().to_io_writer_pretty(writer, self, PrettyConfig::new())
    }

    /// Load the config from a file
    pub fn load(reader: impl Read) -> Result<Self, ron::Error> {
        Ok(ron::de::from_reader(reader)?)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            gamepad_configs: Default::default(),
            hotkeys: Default::default(),
            graphics_setting: Default::default(),
            audio_settings: Default::default(),
            file_browser_home_directory: STORAGE_DIRECTORY.clone(),
            log_location: STORAGE_DIRECTORY.join("log"),
            database_location: STORAGE_DIRECTORY.join("database.redb"),
            save_directory: STORAGE_DIRECTORY.join("saves"),
            snapshot_directory: STORAGE_DIRECTORY.join("snapshots"),
            rom_store_directory: STORAGE_DIRECTORY.join("roms"),
        }
    }
}
