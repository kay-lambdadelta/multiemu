use crate::interpolate::Interpolator;
use crate::sample::conversion::IntoSample;
use crate::sample::{Sample, conversion::FromSample};
use nalgebra::{DefaultAllocator, Dim, OVector, allocator::Allocator};
use num::rational::Ratio;

pub trait FrameIterator<S: Sample, CHANNELS: Dim>: Iterator<Item = OVector<S, CHANNELS>>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    fn convert_sample<S2: Sample + FromSample<S>>(self) -> impl FrameIterator<S2, CHANNELS>;
    fn resample<I: Interpolator<S, CHANNELS>>(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        interpolator: I,
    ) -> impl Iterator<Item = OVector<S, CHANNELS>>;
}

impl<S: Sample, CHANNELS: Dim, SourceIterator: Iterator<Item = OVector<S, CHANNELS>>>
    FrameIterator<S, CHANNELS> for SourceIterator
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    fn convert_sample<S2: Sample + FromSample<S>>(self) -> impl FrameIterator<S2, CHANNELS> {
        self.map(|s| s.map(|s| s.into_sample()))
    }

    fn resample<I: Interpolator<S, CHANNELS>>(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        interpolator: I,
    ) -> impl Iterator<Item = OVector<S, CHANNELS>> {
        interpolator.interpolate(source_rate, target_rate, self)
    }
}
