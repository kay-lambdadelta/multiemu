use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct PpuColor {
    pub luminance: u8,
    pub hue: u8,
}
