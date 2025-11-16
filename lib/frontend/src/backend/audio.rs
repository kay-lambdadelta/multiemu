use std::{fmt::Debug, sync::Arc};

use multiemu_runtime::platform::Platform;
use num::rational::Ratio;

use crate::MaybeMachine;

/// Audio runtime to provide the frontend
pub trait AudioRuntime<P: Platform>: Debug {
    /// Initialize it with a pointer to the [`MaybeMachine`]
    fn new(maybe_machine: Arc<MaybeMachine<P>>) -> Self;
    /// Get the used sample rate
    fn sample_rate(&self) -> Ratio<u32>;
    /// Pause audio playback
    fn pause(&self);
    /// Play audio
    fn play(&self);
}
