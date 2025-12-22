use std::io::{Read, Write};

use multiemu_audio::{FrameIterator, SquareWave};
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentVersion, SampleSource},
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    path::MultiemuPath,
    platform::Platform,
    scheduler::{Frequency, Period, SynchronizationContext},
};
use nalgebra::SVector;
use ringbuffer::{AllocRingBuffer, RingBuffer};

#[derive(Debug)]
pub struct Chip8Audio {
    // The CPU will set this according to what the program wants
    timer: u8,
    buffer: AllocRingBuffer<SVector<f32, 1>>,
    wave_generator: SquareWave<f32, 1>,
    processor_frequency: Frequency,
    timer_accumulator: Period,
}

impl Chip8Audio {
    pub fn set(&mut self, value: u8) {
        self.timer = value;
    }
}

impl Component for Chip8Audio {
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);
        let timer = std::array::from_mut(&mut self.timer);

        reader.read_exact(timer)?;

        Ok(())
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let timer = std::array::from_ref(&self.timer);

        writer.write_all(timer)?;

        Ok(())
    }

    fn get_audio_channel(&mut self, _audio_output_path: &MultiemuPath) -> SampleSource<'_> {
        let sample_rate = self.processor_frequency.to_num();

        SampleSource {
            source: Box::new(self.buffer.drain().repeat_last_frame()),
            sample_rate,
        }
    }

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        let timer_period = Period::from_num(60).recip();

        for _ in context.allocate(self.processor_frequency.recip(), None) {
            if self.timer != 0 {
                self.buffer.enqueue(self.wave_generator.next().unwrap());
            }

            self.timer_accumulator += self.processor_frequency.recip();
            while self.timer_accumulator >= timer_period {
                self.timer = self.timer.saturating_sub(1);
                self.timer_accumulator -= timer_period;
            }
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= self.processor_frequency.recip()
    }
}

#[derive(Debug)]
pub struct Chip8AudioConfig {
    pub processor_frequency: Frequency,
}

impl<P: Platform> ComponentConfig<P> for Chip8AudioConfig {
    type Component = Chip8Audio;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        component_builder
            .set_scheduler_participation(SchedulerParticipation::OnDemand)
            .insert_audio_channel("mono");

        Ok(Chip8Audio {
            timer: 0,
            buffer: AllocRingBuffer::new(440),
            wave_generator: SquareWave::new(440.0, self.processor_frequency.to_num(), 0.5),
            processor_frequency: self.processor_frequency,
            timer_accumulator: Period::ZERO,
        })
    }
}
