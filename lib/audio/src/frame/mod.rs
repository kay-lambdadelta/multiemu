use core::cmp::Ordering;
use nalgebra::SVector;
use num::Float;

use crate::{FromSample, Interpolator, SampleFormat, sample::IntoSample};

/// Helper iterator for operating on frames of samples
pub trait FrameIterator<S: SampleFormat, const CHANNELS: usize>:
    Iterator<Item = SVector<S, CHANNELS>>
{
    /// Convert the samples in the iterator to another sample type
    fn rescale<S2: SampleFormat + FromSample<S>>(self) -> impl FrameIterator<S2, CHANNELS>
    where
        Self: Sized;

    /// Use the specified [Interpolator] to resample the iterator
    fn resample<F: Float + SampleFormat>(
        self,
        source_rate: f32,
        target_rate: f32,
        interpolator: impl Interpolator<S, CHANNELS, F>,
    ) -> impl FrameIterator<S, CHANNELS>
    where
        Self: Sized;

    /// Mix the channels of the iterator into a different number of channels
    fn remix<const CHANNELS2: usize>(self) -> impl FrameIterator<S, CHANNELS2>
    where
        Self: Sized;

    /// Normalize the samples in the iterator
    fn normalize(self) -> impl FrameIterator<S, CHANNELS>
    where
        Self: Sized;

    /// Repeat the final frame of the source forever
    fn repeat_last_frame(self) -> impl FrameIterator<S, CHANNELS>
    where
        Self: Sized;
}

impl<S: SampleFormat, const CHANNELS: usize, SourceIterator: Iterator<Item = SVector<S, CHANNELS>>>
    FrameIterator<S, CHANNELS> for SourceIterator
{
    fn rescale<S2: SampleFormat + FromSample<S>>(self) -> impl FrameIterator<S2, CHANNELS> {
        self.map(|s| s.map(|s| s.into_sample()))
    }

    fn resample<F: Float + SampleFormat>(
        self,
        source_rate: f32,
        target_rate: f32,
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

    fn repeat_last_frame(self) -> impl FrameIterator<S, CHANNELS> {
        struct RepeatLastFrame<
            I: Iterator<Item = SVector<S, CHANNELS>>,
            S: SampleFormat,
            const CHANNELS: usize,
        > {
            source: I,
            last_frame: SVector<S, CHANNELS>,
            exhausted: bool,
        }

        impl<I: Iterator<Item = SVector<S, CHANNELS>>, S: SampleFormat, const CHANNELS: usize>
            Iterator for RepeatLastFrame<I, S, CHANNELS>
        {
            type Item = SVector<S, CHANNELS>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.exhausted {
                    Some(self.last_frame)
                } else {
                    match self.source.next() {
                        Some(frame) => {
                            self.last_frame = frame;
                            Some(frame)
                        }
                        None => {
                            self.exhausted = true;
                            Some(self.last_frame)
                        }
                    }
                }
            }
        }

        RepeatLastFrame {
            source: self,
            last_frame: SVector::from_element(S::equilibrium()),
            exhausted: false,
        }
    }
}
