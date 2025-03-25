use super::Interpolator;
use crate::frame::FrameIterator;
use crate::sample::{Sample, conversion::FromSample};
use nalgebra::{ComplexField, SVector};
use num::rational::Ratio;
use num::{Float, ToPrimitive};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

#[derive(Default)]
pub struct Sinc<const WINDOW_SIZE: usize>;

impl<S: Sample, const CHANNELS: usize, F: Float + Sample + ComplexField, const WINDOW_SIZE: usize>
    Interpolator<S, CHANNELS, F> for Sinc<WINDOW_SIZE>
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
        interpolate_internal::<_, CHANNELS, _, WINDOW_SIZE>(source_rate, target_rate, input)
    }
}

impl<S: Sample, const CHANNELS: usize, F: Float + Sample + ComplexField, const WINDOW_SIZE: usize>
    Interpolator<S, CHANNELS, F> for &Sinc<WINDOW_SIZE>
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
        interpolate_internal::<_, CHANNELS, _, WINDOW_SIZE>(source_rate, target_rate, input)
    }
}

#[inline]
fn interpolate_internal<
    S: Sample + FromSample<F>,
    const CHANNELS: usize,
    F: Float + Sample + FromSample<S> + ComplexField,
    const WINDOW_SIZE: usize,
>(
    source_rate: Ratio<u32>,
    target_rate: Ratio<u32>,
    input: impl IntoIterator<Item = SVector<S, CHANNELS>>,
) -> impl FrameIterator<S, CHANNELS> {
    let mut input = input.into_iter().rescale::<F>();
    let mut input_exhausted = false;

    // Initialize the ring buffer with four samples
    let mut held_samples = ConstGenericRingBuffer::new();
    for _ in 0..WINDOW_SIZE {
        if let Some(sample) = input.next() {
            held_samples.push(sample);
        } else {
            input_exhausted = true;
            break;
        }
    }

    for _ in 0..(WINDOW_SIZE - held_samples.len()) {
        held_samples.push(SVector::from_element(F::equilibrium()));
    }

    SincIterator::<F, CHANNELS, _> {
        // TODO: Do this without f64 intermediary
        resampling_ratio: F::from_f64((target_rate / source_rate).to_f64().unwrap()).unwrap(),
        index: F::zero(),
        input_index: F::zero(),
        held_samples,
        input,
        input_exhausted,
    }
    .rescale::<S>()
}

struct SincIterator<
    F: Float + Sample + ComplexField,
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

impl<
    F: Float + Sample + ComplexField,
    const CHANNELS: usize,
    I: Iterator<Item = SVector<F, CHANNELS>>,
> Iterator for SincIterator<F, CHANNELS, I>
{
    type Item = SVector<F, CHANNELS>;

    fn next(&mut self) -> Option<Self::Item> {
        let input_target_index = self.index / self.resampling_ratio;

        while self.input_index < input_target_index {
            if let Some(sample) = self.input.next() {
                self.held_samples.push(sample);
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

        let interpolated_sample = sinc_interpolate(&self.held_samples, fractional_part);
        self.index += F::one();

        Some(interpolated_sample)
    }
}

#[inline]
fn sinc_interpolate<
    F: Float + Sample + ComplexField,
    const CHANNELS: usize,
    const WINDOW_SIZE: usize,
>(
    samples: &ConstGenericRingBuffer<SVector<F, CHANNELS>, WINDOW_SIZE>,
    mu: F,
) -> SVector<F, CHANNELS> {
    let mut result = SVector::from_element(F::equilibrium());

    for i in 0..WINDOW_SIZE {
        let x = mu - F::from_usize(i).unwrap();
        let sinc_value = x.sinc();
        result += samples[i] * sinc_value;
    }

    result
}
