use crossbeam::queue::ArrayQueue;
use multiemu_audio::{
    frame::FrameIterator,
    interpolate::linear::Linear,
    sample::{Sample, conversion::FromSample},
};
use nalgebra::{SVector, Vector2};
use num::rational::Ratio;
use std::sync::Arc;

pub struct AudioQueue
where
    Self: Send + Sync,
{
    pub frames: Arc<ArrayQueue<SVector<i16, 2>>>,
    sample_rate: Ratio<u32>,
}

impl AudioQueue {
    pub fn new(sample_rate: Ratio<u32>) -> Self {
        Self {
            frames: Arc::new(ArrayQueue::new(sample_rate.to_integer() as usize)),
            sample_rate,
        }
    }

    pub fn fetch_frames<S: Sample + FromSample<i16>>(
        &self,
        sample_rate: Ratio<u32>,
        channel_count: usize,
    ) -> impl Iterator<Item = Vector2<S>> {
        let frames = self.frames.clone();

        std::iter::from_fn(move || frames.pop())
            .resample(self.sample_rate, sample_rate, Linear::<f32>::default())
            .convert_sample()
    }

    pub fn push_frames<S: Sample>(&self, frames: impl IntoIterator<Item = SVector<S, 2>>)
    where
        i16: FromSample<S>,
    {
        for frame in frames.into_iter().convert_sample() {
            self.frames.push(frame).unwrap();
        }
    }
}
