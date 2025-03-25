use crossbeam::queue::ArrayQueue;
use multiemu_audio::{
    frame::FrameIterator,
    interpolate::cubic::Cubic,
    sample::{Sample, conversion::FromSample},
};
use nalgebra::SVector;
use num::rational::Ratio;

/// Queue to be shared with components that contains audio data
#[derive(Debug)]
pub struct AudioQueue {
    frames: ArrayQueue<SVector<f32, 2>>,
    sample_rate: Ratio<u32>,
}

impl AudioQueue {
    /// Create a new audio queue
    pub fn new(sample_rate: Ratio<u32>) -> Self {
        Self {
            frames: ArrayQueue::new(sample_rate.to_integer() as usize),
            sample_rate,
        }
    }

    /// Push audio frames
    pub fn extend<S: Sample + FromSample<f32>, const CHANNELS: usize>(
        &self,
        frames: impl IntoIterator<Item = SVector<S, CHANNELS>>,
    ) where
        f32: FromSample<S>,
    {
        for frame in frames.into_iter().rescale().remix() {
            self.frames.force_push(frame);
        }
    }

    pub fn fetch<S: Sample + FromSample<f32>, const CHANNELS: usize>(
        &self,
        target_rate: Ratio<u32>,
        buffer: &mut [SVector<S, CHANNELS>],
    ) where
        f32: FromSample<S>,
    {
        std::iter::from_fn(|| self.frames.pop())
            .chain(std::iter::repeat(SVector::from_element(f32::equilibrium())))
            .resample::<f32>(self.sample_rate, target_rate, Cubic)
            .rescale()
            .remix()
            .fill_buf(buffer);
    }
}
