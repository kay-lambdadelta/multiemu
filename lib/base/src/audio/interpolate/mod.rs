use crate::audio::SampleFormat;
use nalgebra::{ComplexField, SVector};
use num::{Float, rational::Ratio};

mod cubic;
mod linear;
mod sinc;

pub use cubic::Cubic;
pub use linear::Linear;
pub use sinc::Sinc;

/// Trait for interpolators, generic over frame size and sample format
pub trait Interpolator<
    S: SampleFormat,
    const CHANNELS: usize,
    INTERMEDIATE: Float + SampleFormat + ComplexField,
>
{
    /// Interpolates a sequence of samples from a source rate to a target rate given an interpolator
    fn interpolate(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = SVector<S, CHANNELS>>;
}
