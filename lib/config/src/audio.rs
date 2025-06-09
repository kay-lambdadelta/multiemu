use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
/// Interpolation settings the audio backend should use
pub enum Interpolation {
    /// Linear interpolation, lowest quality
    Linear,
    #[default]
    /// Cubic interpolation, mid quality
    Cubic,
    /// Sinc interpolation, highest quality
    Sinc {
        /// Number of taps to use
        taps: u8,
    },
}

#[serde_as]
#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
/// Audio settings
pub struct AudioSettings {
    /// Interpolation settings
    pub interpolation: Interpolation,
}
