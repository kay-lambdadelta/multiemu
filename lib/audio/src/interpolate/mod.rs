use nalgebra::SVector;
use num::Float;

use crate::SampleFormat;

mod cubic;
mod linear;

pub use cubic::Cubic;
pub use linear::Linear;

/// Trait for interpolators, generic over frame size and sample format
pub trait Interpolator<S: SampleFormat, const CHANNELS: usize, INTERMEDIATE: Float + SampleFormat> {
    /// Interpolates a sequence of samples from a source rate to a target rate
    /// given an interpolator
    fn interpolate(
        self,
        source_rate: f32,
        target_rate: f32,
        input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = SVector<S, CHANNELS>>;
}
