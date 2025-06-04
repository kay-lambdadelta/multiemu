use nalgebra::SVector;
use num::rational::Ratio;
use crate::audio::sample::Sample;

pub mod frame;
pub mod interpolate;
pub mod sample;

pub trait AudioDataCallback<S: Sample>: Send + Sync + 'static {
    fn sample_rate(&self) -> Ratio<u32>;

    fn generate_audio(&self, buffer: &mut [SVector<S, 2>]);
}
