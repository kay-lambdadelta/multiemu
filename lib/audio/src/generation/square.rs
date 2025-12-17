use nalgebra::SVector;

use crate::SampleFormat;

#[derive(Debug)]
/// Square wave generator
pub struct SquareWave<S: SampleFormat, const CHANNELS: usize> {
    step: f32,
    phase: f32,
    amplitude: S,
}

impl<S: SampleFormat, const CHANNELS: usize> SquareWave<S, CHANNELS> {
    /// Create the square wave
    pub fn new(frequency: f32, sample_rate: f32, amplitude: S) -> Self {
        let step = frequency / sample_rate;

        Self {
            step,
            phase: 0.0,
            amplitude,
        }
    }
}

impl<S: SampleFormat, const CHANNELS: usize> Iterator for SquareWave<S, CHANNELS> {
    type Item = SVector<S, CHANNELS>;

    fn next(&mut self) -> Option<Self::Item> {
        // A square wave toggles every 0.5 in phase
        let value = if self.phase < 0.5 {
            S::equilibrium() + self.amplitude
        } else {
            S::equilibrium() - self.amplitude
        };

        self.phase = (self.phase + self.step) % 1.0;
        Some(SVector::<S, CHANNELS>::from_element(value))
    }
}
