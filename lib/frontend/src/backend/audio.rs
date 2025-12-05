use std::fmt::Debug;

use multiemu_runtime::platform::Platform;
use num::rational::Ratio;

/// Audio runtime to provide the frontend
pub trait AudioRuntime<P: Platform>: Debug {
    /// Initialize it with a pointer to the [`MaybeMachine`]
    fn new() -> Self;
    /// Get the used sample rate
    fn sample_rate(&self) -> Ratio<u32>;
    /// Pause audio playback
    fn pause(&self);
    /// Play audio
    fn play(&self);
}
