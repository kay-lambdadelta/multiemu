use std::time::Duration;

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
#[derive(Serialize, Deserialize, Debug, Clone)]
/// Audio settings
pub struct AudioSettings {
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    #[serde_inline_default(Duration::from_millis(100))]
    /// How much audio will the audio queue hold
    pub latency: Duration,
    /// Interpolation settings
    pub interpolation: Interpolation,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            latency: Duration::from_millis(100),
            interpolation: Interpolation::default(),
        }
    }
}
