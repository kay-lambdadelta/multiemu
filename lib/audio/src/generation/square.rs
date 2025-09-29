use crate::SampleFormat;
use nalgebra::SVector;
use num::rational::Ratio;

#[derive(Debug)]
/// Square wave generator
pub struct SquareWave<S: SampleFormat, const CHANNELS: usize> {
    step: Ratio<u32>,
    phase: Ratio<u32>,
    amplitude: S,
}

impl<S: SampleFormat, const CHANNELS: usize> SquareWave<S, CHANNELS> {
    /// Create the square wave
    pub fn new(frequency: Ratio<u32>, sample_rate: Ratio<u32>, amplitude: S) -> Self {
        let step = frequency / sample_rate;

        Self {
            step,
            phase: Ratio::from_integer(0),
            amplitude,
        }
    }
}

impl<S: SampleFormat, const CHANNELS: usize> Iterator for SquareWave<S, CHANNELS> {
    type Item = SVector<S, CHANNELS>;

    fn next(&mut self) -> Option<Self::Item> {
        // A square wave toggles every 0.5 in phase
        let value = if self.phase < Ratio::new(1, 2) {
            S::equilibrium() + self.amplitude
        } else {
            S::equilibrium() - self.amplitude
        };

        self.phase = (self.phase + self.step) % Ratio::from_integer(1);
        Some(SVector::<S, CHANNELS>::from_element(value))
    }
}
