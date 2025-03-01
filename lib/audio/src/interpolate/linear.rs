use super::Interpolator;
use crate::frame::FrameIterator;
use crate::sample::Sample;
use crate::sample::conversion::FromSample;
use nalgebra::allocator::Allocator;
use nalgebra::{DefaultAllocator, Dim, OVector, RawStorage};
use num::rational::Ratio;
use num::{Float, ToPrimitive};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

#[derive(Default)]
pub struct Linear<F: Float + Sample = f32> {
    _precision: std::marker::PhantomData<F>,
}

impl<S: Sample, CHANNELS: Dim, F: Float + Sample> Interpolator<S, CHANNELS> for Linear<F>
where
    DefaultAllocator: Allocator<CHANNELS>,
    F: FromSample<S>,
    S: FromSample<F>,
{
    fn interpolate(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        input: impl IntoIterator<Item = OVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = OVector<S, CHANNELS>> {
        interpolate_internal(source_rate, target_rate, input)
    }
}

impl<S: Sample, CHANNELS: Dim, F: Float + Sample> Interpolator<S, CHANNELS> for &Linear<F>
where
    DefaultAllocator: Allocator<CHANNELS>,
    F: FromSample<S>,
    S: FromSample<F>,
{
    fn interpolate(
        self,
        source_rate: Ratio<u32>,
        target_rate: Ratio<u32>,
        input: impl IntoIterator<Item = OVector<S, CHANNELS>>,
    ) -> impl Iterator<Item = OVector<S, CHANNELS>> {
        interpolate_internal(source_rate, target_rate, input)
    }
}

#[inline]
fn interpolate_internal<
    S: Sample + FromSample<F>,
    CHANNELS: Dim,
    F: Float + Sample + FromSample<S>,
>(
    source_rate: Ratio<u32>,
    target_rate: Ratio<u32>,
    input: impl IntoIterator<Item = OVector<S, CHANNELS>>,
) -> impl FrameIterator<S, CHANNELS>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    let mut input = input.into_iter().convert_sample::<F>();
    let mut input_exhausted = false;

    let mut held_samples = ConstGenericRingBuffer::new();

    for _ in 0..2 {
        if let Some(sample) = input.next() {
            held_samples.push(sample);
        } else {
            input_exhausted = true;
            break;
        }
    }

    let shape = held_samples[0].data.shape();

    for _ in 0..(2 - held_samples.len()) {
        held_samples.push(OVector::from_element_generic(
            shape.0,
            shape.1,
            F::equilibrium(),
        ));
    }

    LinearIterator::<F, CHANNELS, _> {
        resampling_ratio: F::from_f64((target_rate / source_rate).to_f64().unwrap()).unwrap(),
        index: F::zero(),
        input_index: F::zero(),
        held_samples,
        input,
        input_exhausted,
    }
    .convert_sample::<S>()
}

struct LinearIterator<F: Float + Sample, CHANNELS: Dim, I: Iterator<Item = OVector<F, CHANNELS>>>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    resampling_ratio: F,
    index: F,
    input_index: F,
    held_samples: ConstGenericRingBuffer<OVector<F, CHANNELS>, 2>,
    input: I,
    input_exhausted: bool,
}

impl<F: Float + Sample, CHANNELS: Dim, I: Iterator<Item = OVector<F, CHANNELS>>> Iterator
    for LinearIterator<F, CHANNELS, I>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    type Item = OVector<F, CHANNELS>;

    fn next(&mut self) -> Option<Self::Item> {
        let input_target_index = self.index / self.resampling_ratio;

        // Try to take samples
        while self.input_index < input_target_index {
            if let Some(sample) = self.input.next() {
                self.held_samples.push(sample);
            } else {
                self.input_exhausted = true;
                break;
            }

            self.input_index += F::one();
        }

        // Exit if we've reached the end
        if self.input_exhausted && self.input_index <= input_target_index {
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
