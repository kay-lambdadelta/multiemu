use super::Interpolator;
use crate::frame::FrameIterator;
use crate::sample::{Sample, conversion::FromSample};
use nalgebra::{DefaultAllocator, Dim, OVector, RawStorage, allocator::Allocator};
use num::rational::Ratio;
use num::{Float, ToPrimitive};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

#[derive(Default)]
pub struct Cubic<F: Float + Sample = f32> {
    _precision: std::marker::PhantomData<F>,
}

impl<S: Sample, CHANNELS: Dim, F: Float + Sample> Interpolator<S, CHANNELS> for Cubic<F>
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

impl<S: Sample, CHANNELS: Dim, F: Float + Sample> Interpolator<S, CHANNELS> for &Cubic<F>
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

    // Initialize the ring buffer with four samples
    let mut held_samples = ConstGenericRingBuffer::new();
    for _ in 0..4 {
        if let Some(sample) = input.next() {
            held_samples.push(sample);
        } else {
            input_exhausted = true;
            break;
        }
    }

    if let Some(shape) = held_samples.get(0).map(|s| s.data.shape()) {
        for _ in 0..(4 - held_samples.len()) {
            held_samples.push(OVector::from_element_generic(
                shape.0,
                shape.1,
                F::equilibrium(),
            ));
        }
    }

    CubicIterator::<F, CHANNELS, _> {
        // TODO: Do this without f64 intermediary
        resampling_ratio: F::from_f64((target_rate / source_rate).to_f64().unwrap()).unwrap(),
        index: F::zero(),
        input_index: F::zero(),
        held_samples,
        input,
        input_exhausted,
    }
    .convert_sample::<S>()
}

struct CubicIterator<F: Float + Sample, CHANNELS: Dim, I: Iterator<Item = OVector<F, CHANNELS>>>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    resampling_ratio: F,
    index: F,
    input_index: F,
    held_samples: ConstGenericRingBuffer<OVector<F, CHANNELS>, 4>,
    input: I,
    input_exhausted: bool,
}

impl<F: Float + Sample, CHANNELS: Dim, I: Iterator<Item = OVector<F, CHANNELS>>> Iterator
    for CubicIterator<F, CHANNELS, I>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    type Item = OVector<F, CHANNELS>;

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
fn cubic_interpolate<F: Float + Sample, CHANNELS: Dim>(
    y0: &OVector<F, CHANNELS>,
    y1: &OVector<F, CHANNELS>,
    y2: &OVector<F, CHANNELS>,
    y3: &OVector<F, CHANNELS>,
    mu: F,
) -> OVector<F, CHANNELS>
where
    DefaultAllocator: Allocator<CHANNELS>,
{
    let mu2 = mu.powi(2);
    let a0 = y3 - y2 - y0 + y1;
    let a1 = y0 - y1 - a0.clone();
    let a2 = y2 - y0;
    let a3 = y1;

    a0 * mu * mu2 + a1 * mu2 + a2 * mu + a3
}
