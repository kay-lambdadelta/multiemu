use crate::audio::sample::Sample;
use nalgebra::{ComplexField, SVector};
use num::{Float, rational::Ratio};

pub mod cubic;
pub mod linear;
pub mod sinc;

/// Trait for interpolators, generic over frame size and sample format
pub trait Interpolator<
    S: Sample,
    const CHANNELS: usize,
    INTERMEDIATE: Float + Sample + ComplexField,
>
{
    fn interpolate(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = SVector<S, CHANNELS>>;
}
