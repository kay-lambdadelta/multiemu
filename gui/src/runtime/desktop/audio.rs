use crate::runtime::{AudioRuntime, MaybeMachine};
use bytemuck::Pod;
use cpal::{
    Device, Host, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use multiemu_audio::{FrameIterator, FromSample, Sample};
use nalgebra::SVector;
use num::rational::Ratio;
use std::{fmt::Debug, ops::Deref, sync::Arc};

#[allow(unused)]
pub struct CpalAudioRuntime {
    host: Host,
    device: Device,
    stream: Stream,
    sample_rate: Ratio<u32>,
}

impl Debug for CpalAudioRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalAudio").finish()
    }
}

impl AudioRuntime for CpalAudioRuntime {
    fn new(machine: Arc<MaybeMachine>) -> Self {
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

        let config = device.default_output_config().unwrap();
        tracing::info!("Selected audio device with config: {:#?}", config);

        let sample_rate = Ratio::from_integer(config.sample_rate().0);

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => build_output_stream::<i8>(&device, config.into(), machine),
            cpal::SampleFormat::I16 => build_output_stream::<i16>(&device, config.into(), machine),
            cpal::SampleFormat::I32 => build_output_stream::<i32>(&device, config.into(), machine),
            cpal::SampleFormat::I64 => build_output_stream::<i64>(&device, config.into(), machine),
            cpal::SampleFormat::U8 => build_output_stream::<u8>(&device, config.into(), machine),
            cpal::SampleFormat::U16 => build_output_stream::<u16>(&device, config.into(), machine),
            cpal::SampleFormat::U32 => build_output_stream::<u32>(&device, config.into(), machine),
            cpal::SampleFormat::U64 => build_output_stream::<u64>(&device, config.into(), machine),
            cpal::SampleFormat::F32 => build_output_stream::<f32>(&device, config.into(), machine),
            cpal::SampleFormat::F64 => build_output_stream::<f64>(&device, config.into(), machine),
            _ => unimplemented!(),
        };

        Self {
            host,
            device,
            stream,
            sample_rate,
        }
    }

    fn pause(&self) {
        self.stream.pause().unwrap();
    }

    fn play(&self) {
        self.stream.play().unwrap();
    }

    fn sample_rate(&self) -> Ratio<u32> {
        self.sample_rate
    }
}

pub fn build_output_stream<S: Sample + FromSample<f32> + Pod + cpal::SizedSample>(
    device: &Device,
    config: StreamConfig,
    maybe_machine: Arc<MaybeMachine>,
) -> Stream
where
    f32: FromSample<S>,
{
    match config.channels {
        1 => fetch_audio_data_builder::<S, 1>(device, config, maybe_machine),
        2 => fetch_audio_data_builder::<S, 2>(device, config, maybe_machine),
        3 => fetch_audio_data_builder::<S, 3>(device, config, maybe_machine),
        4 => fetch_audio_data_builder::<S, 4>(device, config, maybe_machine),
        5 => fetch_audio_data_builder::<S, 5>(device, config, maybe_machine),
        6 => fetch_audio_data_builder::<S, 6>(device, config, maybe_machine),
        7 => fetch_audio_data_builder::<S, 7>(device, config, maybe_machine),
        8 => fetch_audio_data_builder::<S, 8>(device, config, maybe_machine),
        _ => unimplemented!(),
    }
}

fn fetch_audio_data_builder<S: Sample + FromSample<f32> + Pod + cpal::SizedSample, const C: usize>(
    device: &Device,
    config: StreamConfig,
    machine: Arc<MaybeMachine>,
) -> Stream
where
    f32: FromSample<S>,
{
    device
        .build_output_stream(
            &config,
            move |data: &mut [S], _| {
                let data: &mut [SVector<S, C>] = bytemuck::cast_slice_mut(data);

                let machine_guard = machine.read().unwrap();

                if let Some(machine) = machine_guard.deref() {
                    let audio_data_callbacks = machine.audio_callbacks::<f32>();

                    if let Some(callback) = audio_data_callbacks.first() {
                        for (destination, source) in data
                            .iter_mut()
                            .zip(callback.generate_audio().remix::<C>().rescale())
                        {
                            *destination = source;
                        }
                    }
                }
            },
            move |err| tracing::error!("an error occurred on the output audio stream: {}", err),
            None,
        )
        .unwrap()
}
