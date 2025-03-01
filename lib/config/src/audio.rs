use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum Interpolation {
    Linear,
    #[default]
    Cubic,
}

#[serde_as]
#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioSettings {
    #[serde_as(as = "serde_with::DurationSecondsWithFrac<f64>")]
    #[serde_inline_default(Duration::from_millis(100))]
    pub latency: Duration,
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
