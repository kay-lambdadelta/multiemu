use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
/// Possible hotkeys this emulator could use
#[allow(missing_docs)]
pub enum Hotkey {
    ToggleMenu,
    FastForward,
    LoadSnapshot,
    StoreSnapshot,
    IncrementSnapshotCounter,
    DecrementSnapshotCounter,
}
