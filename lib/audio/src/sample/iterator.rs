use super::{
    Sample,
    conversion::{FromSample, IntoSample},
};

/// Helper trait for samples
pub trait SampleIterator<S: Sample>: Iterator<Item = S> {
    /// Converts the samples in the iterator to a different sample type
    fn rescale<S2: Sample + FromSample<S>>(self) -> impl SampleIterator<S2>;

    /// Clamps the sample, should be done after every set of operations
    fn normalize(self) -> impl SampleIterator<S>;
}

impl<S: Sample, I: Iterator<Item = S>> SampleIterator<S> for I {
    fn rescale<S2: Sample + FromSample<S>>(self) -> impl SampleIterator<S2> {
        self.map(|s| s.into_sample())
    }

    fn normalize(self) -> impl SampleIterator<S> {
        self.map(|s| s.normalize())
    }
}
