use crate::sample::Sample;
use nalgebra::{DefaultAllocator, Dim, OVector, allocator::Allocator};
use num::rational::Ratio;

pub mod cubic;
pub mod linear;

pub trait Interpolator<S: Sample, CHANNELS: Dim>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    fn interpolate(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        input: impl IntoIterator<Item = OVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = OVector<S, CHANNELS>>;
}
