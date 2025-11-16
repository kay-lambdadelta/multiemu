use std::{fmt::Debug, ops::Deref, sync::Arc};

use bytemuck::Pod;
use cpal::{
    Device, Host, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use itertools::Itertools;
use multiemu_audio::{FrameIterator, FromSample, SampleFormat};
use multiemu_frontend::{AudioRuntime, MaybeMachine};
use multiemu_runtime::platform::Platform;
use nalgebra::SVector;
use num::rational::Ratio;
use ringbuffer::RingBuffer;

#[allow(unused)]
pub struct CpalAudioRuntime<P: Platform> {
    host: Host,
    device: Device,
    stream: Stream,
    sample_rate: Ratio<u32>,
    maybe_machine: Arc<MaybeMachine<P>>,
}

impl<P: Platform> Debug for CpalAudioRuntime<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalAudioRuntime").finish()
    }
}

impl<P: Platform> AudioRuntime<P> for CpalAudioRuntime<P> {
    fn new(maybe_machine: Arc<MaybeMachine<P>>) -> Self {
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

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => {
                build_stream::<P, i8>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::I16 => {
                build_stream::<P, i16>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::I32 => {
                build_stream::<P, i32>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::I64 => {
                build_stream::<P, i64>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U8 => {
                build_stream::<P, u8>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U16 => {
                build_stream::<P, u16>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U32 => {
                build_stream::<P, u32>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U64 => {
                build_stream::<P, u64>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::F32 => {
                build_stream::<P, f32>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::F64 => {
                build_stream::<P, f64>(&device, config.into(), maybe_machine.clone())
            }
            _ => unimplemented!(),
        };

        Self {
            host,
            device,
            stream,
            sample_rate,
            maybe_machine,
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

fn build_stream<P: Platform, OS: SampleFormat + cpal::SizedSample + FromSample<f32> + Pod>(
    device: &Device,
    config: StreamConfig,
    maybe_machine: Arc<MaybeMachine<P>>,
) -> Stream {
    device
        .build_output_stream(
            &config,
            move |data: &mut [OS], _| {
                let data: &mut [SVector<OS, 2>] = bytemuck::cast_slice_mut(data);

                let Ok(maybe_machine) = maybe_machine.read() else {
                    // We have been poisoned. exit.
                    return;
                };

                if let Some(machine) = maybe_machine.deref() {
                    if let Some(audio_resource_path) = machine.audio_outputs.iter().next() {
                        machine
                            .component_registry
                            .interact_dyn_mut(&audio_resource_path.component, |component| {
                                let buffer = component.drain_samples(audio_resource_path);
                                let final_sample =
                                    buffer.into_iter().last().copied().map_or(
                                        SVector::from_element(OS::equilibrium()),
                                        |sample| sample.map(FromSample::from_sample),
                                    );

                                for (destination, source) in data.iter_mut().zip(
                                    buffer
                                        .drain()
                                        .rescale::<OS>()
                                        .chain(std::iter::repeat(final_sample)),
                                ) {
                                    *destination = source;
                                }
                            })
                            .unwrap();
                    }
                } else {
                    data.fill(SVector::from_element(SampleFormat::equilibrium()));
                }
            },
            move |err| tracing::error!("an error occurred on the output audio stream: {}", err),
            None,
        )
        .unwrap()
}
