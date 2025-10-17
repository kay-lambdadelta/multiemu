use crate::sample::IntoSample;
use crate::{FromSample, Interpolator, SampleFormat};
use core::cmp::Ordering;
use nalgebra::{ComplexField, SVector};
use num::{Float, rational::Ratio};

/// Helper iterator for operating on frames of samples
pub trait FrameIterator<S: SampleFormat, const CHANNELS: usize>:
    Iterator<Item = SVector<S, CHANNELS>>
{
    /// Convert the samples in the iterator to another sample type
    fn rescale<S2: SampleFormat + FromSample<S>>(self) -> impl FrameIterator<S2, CHANNELS>;

    /// Use the specified [Interpolator] to resample the iterator
    fn resample<F: Float + SampleFormat + ComplexField>(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        interpolator: impl Interpolator<S, CHANNELS, F>,
    ) -> impl FrameIterator<S, CHANNELS>;

    /// Mix the channels of the iterator into a different number of channels
    fn remix<const CHANNELS2: usize>(self) -> impl FrameIterator<S, CHANNELS2>;

    /// Normalize the samples in the iterator
    fn normalize(self) -> impl FrameIterator<S, CHANNELS>;
}

impl<S: SampleFormat, const CHANNELS: usize, SourceIterator: Iterator<Item = SVector<S, CHANNELS>>>
    FrameIterator<S, CHANNELS> for SourceIterator
{
    fn rescale<S2: SampleFormat + FromSample<S>>(self) -> impl FrameIterator<S2, CHANNELS> {
        self.map(|s| s.map(|s| s.into_sample()))
    }

    fn resample<F: Float + SampleFormat + ComplexField>(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        interpolator: impl Interpolator<S, CHANNELS, F>,
    ) -> impl FrameIterator<S, CHANNELS> {
        interpolator.interpolate(source_rate, target_rate, self)
    }

    fn remix<const CHANNELS2: usize>(self) -> impl FrameIterator<S, CHANNELS2> {
        self.map(move |frame| {
            let mut new_frame = SVector::<S, CHANNELS2>::from_element(S::equilibrium());

            match CHANNELS.cmp(&CHANNELS2) {
                Ordering::Less => {
                    for i in 0..CHANNELS2 {
                        new_frame[i] = frame[i % CHANNELS];
                    }
                }
                Ordering::Equal => {
                    for i in 0..CHANNELS2 {
                        new_frame[i] = frame[i];
                    }
                }
                Ordering::Greater => {
                    for i in 0..CHANNELS2 {
                        let mut sum = S::zero();
                        for j in 0..CHANNELS / CHANNELS2 {
                            sum += frame[i * (CHANNELS / CHANNELS2) + j];
                        }
                        new_frame[i] = sum / S::from_usize(CHANNELS / CHANNELS2).unwrap();
                    }
                }
            }

            new_frame
        })
    }

    fn normalize(self) -> impl FrameIterator<S, CHANNELS> {
        self.map(|s| s.map(|s| s.normalize()))
    }
}
