use std::fmt::Debug;

use cpal::{
    Device, Host,
    traits::{DeviceTrait, HostTrait},
};
use itertools::Itertools;
use multiemu_frontend::AudioRuntime;
use multiemu_runtime::platform::Platform;
use num::rational::Ratio;

#[allow(unused)]
pub struct CpalAudioRuntime {
    host: Host,
    device: Device,
    sample_rate: Ratio<u32>,
}

impl Debug for CpalAudioRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalAudioRuntime").finish()
    }
}

impl<P: Platform> AudioRuntime<P> for CpalAudioRuntime {
    fn new() -> Self {
        let host = cpal::default_host();
        tracing::info!("Selecting audio api {:?}", host.id());

        let device = host
            .default_output_device()
            .expect("failed to get default output device");

        if let Ok(name) = device.name() {
            tracing::info!("Selected audio device with name: {}", name);
        } else {
            tracing::info!("Selected audio device");
        }

        let sample_rate = device.default_output_config().unwrap().sample_rate();
        let config = device
            .supported_output_configs()
            .unwrap()
            .sorted_by_key(|config| config.sample_format() == cpal::SampleFormat::F32)
            .rev()
            .find(|config| config.channels() == 2)
            .unwrap()
            .with_sample_rate(sample_rate);

        tracing::info!("Selected audio device with config: {:#?}", config);

        let sample_rate = Ratio::from_integer(sample_rate.0);

        Self {
            host,
            device,
            sample_rate,
        }
    }

    fn pause(&self) {}

    fn play(&self) {}

    fn sample_rate(&self) -> Ratio<u32> {
        self.sample_rate
    }
}
