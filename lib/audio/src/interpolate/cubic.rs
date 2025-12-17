use nalgebra::SVector;
use num::Float;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

use super::Interpolator;
use crate::{FrameIterator, FromSample, SampleFormat};

#[derive(Default)]
/// Cubic interpolation
pub struct Cubic;

impl<S: SampleFormat, const CHANNELS: usize, F: Float + SampleFormat> Interpolator<S, CHANNELS, F>
    for Cubic
where
    F: FromSample<S>,
    S: FromSample<F>,
{
    fn interpolate(
        self,
        source_rate: f32,
        target_rate: f32,
        input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = SVector<S, CHANNELS>> {
        interpolate_internal(source_rate, target_rate, input)
    }
}

impl<S: SampleFormat, const CHANNELS: usize, F: Float + SampleFormat> Interpolator<S, CHANNELS, F>
    for &Cubic
where
    F: FromSample<S>,
    S: FromSample<F>,
{
    fn interpolate(
        self,
        source_rate: f32,
        target_rate: f32,
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
    source_rate: f32,
    target_rate: f32,
    input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
) -> impl FrameIterator<S, CHANNELS> {
    let mut input = input.into_iter().rescale::<F>();
    let mut input_exhausted = false;

    // Initialize the ring buffer with four samples
    let mut held_samples = ConstGenericRingBuffer::new();
    for _ in 0..4 {
        if let Some(sample) = input.next() {
            held_samples.enqueue(sample);
        } else {
            input_exhausted = true;
            break;
        }
    }

    for _ in 0..(4 - held_samples.len()) {
        held_samples.enqueue(SVector::from_element(F::equilibrium()));
    }

    CubicIterator::<F, CHANNELS, _> {
        resampling_ratio: F::from_f32(target_rate / source_rate).unwrap(),
        index: F::zero(),
        input_index: F::zero(),
        held_samples,
        input,
        input_exhausted,
    }
    .rescale::<S>()
}

struct CubicIterator<
    F: Float + SampleFormat,
    const CHANNELS: usize,
    I: Iterator<Item = SVector<F, CHANNELS>>,
> {
    resampling_ratio: F,
    index: F,
    input_index: F,
    held_samples: ConstGenericRingBuffer<SVector<F, CHANNELS>, 4>,
    input: I,
    input_exhausted: bool,
}

impl<F: Float + SampleFormat, const CHANNELS: usize, I: Iterator<Item = SVector<F, CHANNELS>>>
    Iterator for CubicIterator<F, CHANNELS, I>
{
    type Item = SVector<F, CHANNELS>;

    fn next(&mut self) -> Option<Self::Item> {
        let input_target_index = self.index / self.resampling_ratio;

        while self.input_index < input_target_index {
            if let Some(sample) = self.input.next() {
                self.held_samples.enqueue(sample);
            } else {
                self.input_exhausted = true;
                break;
            }

            self.input_index += F::one();
        }

        if (self.input_exhausted && self.input_index <= input_target_index)
            || self.held_samples.len() < 4
        {
            return None;
        }

        let fractional_part =
            (input_target_index - (self.input_index - F::one())).clamp(F::zero(), F::one());

        let interpolated_sample = cubic_interpolate(
            &self.held_samples[0],
            &self.held_samples[1],
            &self.held_samples[2],
            &self.held_samples[3],
            fractional_part,
        );
        self.index += F::one();

        Some(interpolated_sample)
    }
}

#[inline]
fn cubic_interpolate<F: Float + SampleFormat, const CHANNELS: usize>(
    y0: &SVector<F, CHANNELS>,
    y1: &SVector<F, CHANNELS>,
    y2: &SVector<F, CHANNELS>,
    y3: &SVector<F, CHANNELS>,
    mu: F,
) -> SVector<F, CHANNELS> {
    let mu2 = mu.powi(2);
    let a0 = y3 - y2 - y0 + y1;
    let a1 = y0 - y1 - a0;
    let a2 = y2 - y0;
    let a3 = y1;

    a0 * mu * mu2 + a1 * mu2 + a2 * mu + a3
}
