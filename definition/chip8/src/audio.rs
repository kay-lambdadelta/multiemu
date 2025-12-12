use std::io::{Read, Write};

use multiemu_audio::SquareWave;
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentVersion, SynchronizationContext},
    machine::builder::{ComponentBuilder, SchedulerParticipation},
    path::MultiemuPath,
    platform::Platform,
    scheduler::Period,
};
use nalgebra::SVector;
use num::rational::Ratio;
use ringbuffer::AllocRingBuffer;

#[derive(Debug)]
pub struct Chip8Audio {
    // The CPU will set this according to what the program wants
    timer: u8,
    buffer: AllocRingBuffer<SVector<f32, 2>>,
    wave_generator: SquareWave<f32, 2>,
    host_sample_rate: Ratio<u32>,
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

    fn drain_samples(
        &mut self,
        _audio_output_path: &MultiemuPath,
    ) -> &mut AllocRingBuffer<SVector<f32, 2>> {
        &mut self.buffer
    }

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        while context.allocate_period(Period::ONE / 60) {
            self.timer = self.timer.saturating_sub(1);
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= Period::ONE / 60
    }
}

#[derive(Debug, Default)]
pub struct Chip8AudioConfig;

impl<P: Platform> ComponentConfig<P> for Chip8AudioConfig {
    type Component = Chip8Audio;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let host_sample_rate = component_builder.host_sample_rate();

        component_builder
            .set_scheduler_participation(SchedulerParticipation::OnDemand)
            .insert_audio_output("audio-output");

        Ok(Chip8Audio {
            timer: 0,
            buffer: AllocRingBuffer::new(host_sample_rate.to_integer() as usize),
            wave_generator: SquareWave::new(Ratio::from_integer(440), host_sample_rate, 0.5),
            host_sample_rate,
        })
    }
}
