use super::Interpolator;
use crate::{FrameIterator, FromSample, SampleFormat};
use nalgebra::{ComplexField, SVector};
use num::{Float, ToPrimitive, rational::Ratio};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

#[derive(Default)]
/// Linear interpolation
pub struct Linear;

impl<S: SampleFormat, const CHANNELS: usize, F: Float + SampleFormat + ComplexField>
    Interpolator<S, CHANNELS, F> for Linear
where
    F: FromSample<S>,
    S: FromSample<F>,
{
    fn interpolate(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = SVector<S, CHANNELS>> {
        interpolate_internal(source_rate, target_rate, input)
    }
}

impl<S: SampleFormat, const CHANNELS: usize, F: Float + SampleFormat + ComplexField>
    Interpolator<S, CHANNELS, F> for &Linear
where
    F: FromSample<S>,
    S: FromSample<F>,
{
    fn interpolate(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = SVector<S, CHANNELS>> {
        interpolate_internal(source_rate, target_rate, input)
    }
}

#[inline]
fn interpolate_internal<
    S: SampleFormat + FromSample<F>,
    const CHANNELS: usize,
    F: Float + SampleFormat + FromSample<S>,
>(
    source_rate: Ratio<u32>,
    target_rate: Ratio<u32>,
    input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
) -> impl FrameIterator<S, CHANNELS> {
    let mut input = input.into_iter().rescale::<F>();
    let mut input_exhausted = false;

    let mut held_samples = ConstGenericRingBuffer::new();

    for _ in 0..2 {
        if let Some(sample) = input.next() {
            held_samples.enqueue(sample);
        } else {
            input_exhausted = true;
            break;
        }
    }

    for _ in 0..(2 - held_samples.len()) {
        held_samples.enqueue(SVector::from_element(F::equilibrium()));
    }

    LinearIterator::<F, CHANNELS, _> {
        resampling_ratio: F::from_f64((target_rate / source_rate).to_f64().unwrap()).unwrap(),
        index: F::zero(),
        input_index: F::zero(),
        held_samples,
        input,
        input_exhausted,
    }
    .rescale::<S>()
}

struct LinearIterator<
    F: Float + SampleFormat,
    const CHANNELS: usize,
    I: Iterator<Item = SVector<F, CHANNELS>>,
> {
    resampling_ratio: F,
    index: F,
    input_index: F,
    held_samples: ConstGenericRingBuffer<SVector<F, CHANNELS>, 2>,
    input: I,
    input_exhausted: bool,
}

impl<F: Float + SampleFormat, const CHANNELS: usize, I: Iterator<Item = SVector<F, CHANNELS>>>
    Iterator for LinearIterator<F, CHANNELS, I>
{
    type Item = SVector<F, CHANNELS>;

    fn next(&mut self) -> Option<Self::Item> {
        let input_target_index = self.index / self.resampling_ratio;

        // Try to take samples
        while self.input_index < input_target_index {
            if let Some(sample) = self.input.next() {
                self.held_samples.enqueue(sample);
            } else {
                self.input_exhausted = true;
                break;
            }

            self.input_index += F::one();
        }

        // Exit if we've reached the end
        if (self.input_exhausted && self.input_index <= input_target_index)
            || self.held_samples.len() < 2
        {
            return None;
        }

        // LERP
        let fractional_part =
            (input_target_index - (self.input_index - F::one())).clamp(F::zero(), F::one());

        let interpolated_sample = self.held_samples[0].lerp(&self.held_samples[1], fractional_part);
        self.index += F::one();

        // Convert back to the original sample type
        Some(interpolated_sample)
    }
}
