use bytemuck::Pod;
use cpal::{
    Device, Host, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use multiemu_audio::{FrameIterator, FromSample, SampleFormat};
use multiemu_frontend::{AudioContext, MaybeMachine};
use multiemu_runtime::platform::Platform;
use nalgebra::SVector;
use num::rational::Ratio;
use std::{fmt::Debug, ops::Deref, sync::Arc};

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

impl<P: Platform> AudioContext<P> for CpalAudioRuntime<P>
where
    u8: FromSample<P::SampleFormat>,
    i8: FromSample<P::SampleFormat>,
    u16: FromSample<P::SampleFormat>,
    i16: FromSample<P::SampleFormat>,
    u32: FromSample<P::SampleFormat>,
    i32: FromSample<P::SampleFormat>,
    u64: FromSample<P::SampleFormat>,
    i64: FromSample<P::SampleFormat>,
    f32: FromSample<P::SampleFormat>,
    f64: FromSample<P::SampleFormat>,
{
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

        let config = device.default_output_config().unwrap();
        tracing::info!("Selected audio device with config: {:#?}", config);

        let sample_rate = Ratio::from_integer(config.sample_rate().0);

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => {
                build_output_stream::<_, i8>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::I16 => {
                build_output_stream::<_, i16>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::I32 => {
                build_output_stream::<_, i32>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::I64 => {
                build_output_stream::<_, i64>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U8 => {
                build_output_stream::<_, u8>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U16 => {
                build_output_stream::<_, u16>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U32 => {
                build_output_stream::<_, u32>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::U64 => {
                build_output_stream::<_, u64>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::F32 => {
                build_output_stream::<_, f32>(&device, config.into(), maybe_machine.clone())
            }
            cpal::SampleFormat::F64 => {
                build_output_stream::<_, f64>(&device, config.into(), maybe_machine.clone())
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

pub fn build_output_stream<
    P: Platform,
    OS: SampleFormat + SizedSample + Pod + FromSample<P::SampleFormat>,
>(
    device: &Device,
    config: StreamConfig,
    maybe_machine: Arc<MaybeMachine<P>>,
) -> Stream {
    match config.channels {
        1 => fetch_audio_data_builder::<_, OS, 1>(device, config, maybe_machine),
        2 => fetch_audio_data_builder::<_, OS, 2>(device, config, maybe_machine),
        3 => fetch_audio_data_builder::<_, OS, 3>(device, config, maybe_machine),
        4 => fetch_audio_data_builder::<_, OS, 4>(device, config, maybe_machine),
        5 => fetch_audio_data_builder::<_, OS, 5>(device, config, maybe_machine),
        6 => fetch_audio_data_builder::<_, OS, 6>(device, config, maybe_machine),
        7 => fetch_audio_data_builder::<_, OS, 7>(device, config, maybe_machine),
        8 => fetch_audio_data_builder::<_, OS, 8>(device, config, maybe_machine),
        _ => unimplemented!(),
    }
}

fn fetch_audio_data_builder<
    P: Platform,
    OS: SampleFormat + SizedSample + Pod + FromSample<P::SampleFormat>,
    const C: usize,
>(
    device: &Device,
    config: StreamConfig,
    maybe_machine: Arc<MaybeMachine<P>>,
) -> Stream {
    device
        .build_output_stream(
            &config,
            move |data: &mut [OS], _| {
                let data: &mut [SVector<OS, C>] = bytemuck::cast_slice_mut(data);

                let Ok(maybe_machine) = maybe_machine.read() else {
                    // We have been poisoned. exit.
                    return;
                };

                if let Some(maybe_machine) = maybe_machine.deref() {
                    if let Some(audio_output_info) = maybe_machine.audio_outputs.values().next() {
                        for (destination, source) in data.iter_mut().zip(
                            audio_output_info
                                .callback
                                .generate_samples()
                                .remix::<C>()
                                .rescale::<OS>(),
                        ) {
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
