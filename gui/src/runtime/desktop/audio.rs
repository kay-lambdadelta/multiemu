use bytemuck::Pod;
use cpal::{
    Device, Host, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use multiemu_audio::sample::{Sample, conversion::FromSample};
use multiemu_machine::audio::AudioQueue;
use nalgebra::SVector;
use num::rational::Ratio;
use std::sync::Arc;

pub struct CpalAudio {
    host: Host,
    device: Device,
    queue: Arc<AudioQueue>,
    _stream: Stream,
}

impl Default for CpalAudio {
    fn default() -> Self {
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

        let queue = Arc::new(AudioQueue::new(Ratio::from_integer(config.sample_rate().0)));

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => {
                build_output_stream::<i8>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::I16 => {
                build_output_stream::<i16>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::I32 => {
                build_output_stream::<i32>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::I64 => {
                build_output_stream::<i64>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::U8 => {
                build_output_stream::<u8>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::U16 => {
                build_output_stream::<u16>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::U32 => {
                build_output_stream::<u32>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::U64 => {
                build_output_stream::<u64>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::F32 => {
                build_output_stream::<f32>(queue.clone(), &device, config.into())
            }
            cpal::SampleFormat::F64 => {
                build_output_stream::<f64>(queue.clone(), &device, config.into())
            }
            _ => unimplemented!(),
        };

        Self {
            host,
            device,
            queue,
            _stream: stream,
        }
    }
}

pub fn build_output_stream<S: Sample + FromSample<f32> + Pod + cpal::SizedSample>(
    queue: Arc<AudioQueue>,
    device: &Device,
    config: StreamConfig,
) -> Stream
where
    f32: FromSample<S>,
{
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [S], _| match config.channels {
                1 => {
                    let data: &mut [SVector<S, 1>] = bytemuck::try_cast_slice_mut(data).unwrap();
                    queue.fetch(Ratio::from_integer(config.sample_rate.0), data);
                }
                2 => {
                    let data: &mut [SVector<S, 2>] = bytemuck::try_cast_slice_mut(data).unwrap();
                    queue.fetch(Ratio::from_integer(config.sample_rate.0), data);
                }
                4 => {
                    let data: &mut [SVector<S, 4>] = bytemuck::try_cast_slice_mut(data).unwrap();
                    queue.fetch(Ratio::from_integer(config.sample_rate.0), data);
                }
                5 => {
                    let data: &mut [SVector<S, 5>] = bytemuck::try_cast_slice_mut(data).unwrap();
                    queue.fetch(Ratio::from_integer(config.sample_rate.0), data);
                }
                7 => {
                    let data: &mut [SVector<S, 7>] = bytemuck::try_cast_slice_mut(data).unwrap();
                    queue.fetch(Ratio::from_integer(config.sample_rate.0), data);
                }
                _ => unimplemented!(),
            },
            move |err| tracing::error!("an error occurred on the output audio stream: {}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();

    stream
}
