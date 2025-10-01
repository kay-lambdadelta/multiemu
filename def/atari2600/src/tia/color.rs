use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct TiaColor {
    pub luminance: u8,
    pub hue: u8,
}
