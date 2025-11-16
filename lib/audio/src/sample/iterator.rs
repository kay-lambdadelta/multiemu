use nalgebra::SVector;

use super::{
    SampleFormat,
    conversion::{FromSample, IntoSample},
};
use crate::FrameIterator;

/// Helper trait for samples
pub trait SampleIterator<S: SampleFormat>: Iterator<Item = S> {
    /// Converts the samples in the iterator to a different sample type
    fn rescale<S2: SampleFormat + FromSample<S>>(self) -> impl SampleIterator<S2>;

    /// Clamps the sample, should be done after every set of operations
    fn normalize(self) -> impl SampleIterator<S>;

    /// Converts to a single channel frame
    fn map_frame(self) -> impl FrameIterator<S, 1>;
}

impl<S: SampleFormat, I: Iterator<Item = S>> SampleIterator<S> for I {
    fn rescale<S2: SampleFormat + FromSample<S>>(self) -> impl SampleIterator<S2> {
        self.map(|s| s.into_sample())
    }

    fn normalize(self) -> impl SampleIterator<S> {
        self.map(|s| s.normalize())
    }

    fn map_frame(self) -> impl FrameIterator<S, 1> {
        self.map(SVector::from_element)
    }
}
