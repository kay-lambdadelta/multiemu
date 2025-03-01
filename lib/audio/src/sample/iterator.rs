use super::{
    Sample,
    conversion::{FromSample, IntoSample},
};

pub trait SampleIterator<S: Sample>: Iterator<Item = S> {
    fn convert_sample<S2: Sample + FromSample<S>>(self) -> impl SampleIterator<S2>;
}

impl<S: Sample, I: Iterator<Item = S>> SampleIterator<S> for I {
    fn convert_sample<S2: Sample + FromSample<S>>(self) -> impl SampleIterator<S2> {
        self.map(|s| s.into_sample())
    }
}
