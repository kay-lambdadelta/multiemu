use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_audio::{Cubic, FrameIterator, Linear, Sinc};
use nalgebra::Vector2;
use num::rational::Ratio;
use std::hint::black_box;

const BUFFER_SIZE: usize = 4096;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("resample_linear_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(Ratio::from_integer(44100), Ratio::from_integer(440), Linear)
                .collect();
        })
    });

    c.bench_function("resample_linear_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(Ratio::from_integer(440), Ratio::from_integer(44100), Linear)
                .collect();
        })
    });

    c.bench_function("resample_linear_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(Ratio::from_integer(44100), Ratio::from_integer(440), Linear)
                .collect();
        })
    });

    c.bench_function("resample_linear_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(Ratio::from_integer(440), Ratio::from_integer(44100), Linear)
                .collect();
        })
    });

    c.bench_function("resample_cubic_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(Ratio::from_integer(44100), Ratio::from_integer(440), Cubic)
                .collect();
        })
    });

    c.bench_function("resample_cubic_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(Ratio::from_integer(440), Ratio::from_integer(44100), Cubic)
                .collect();
        })
    });

    c.bench_function("resample_cubic_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(Ratio::from_integer(44100), Ratio::from_integer(440), Cubic)
                .collect();
        })
    });

    c.bench_function("resample_cubic_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(Ratio::from_integer(440), Ratio::from_integer(44100), Cubic)
                .collect();
        })
    });

    c.bench_function("resample_sinc1_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<1>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc1_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<1>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc1_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<1>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc1_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<1>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc2_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<2>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc2_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<2>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc2_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<2>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc2_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<2>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc4_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<4>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc4_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<4>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc4_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<4>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc4_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<4>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc8_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<8>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc8_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f32>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<8>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc8_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Sinc::<8>,
                )
                .collect();
        })
    });

    c.bench_function("resample_sinc8_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample::<f64>(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Sinc::<8>,
                )
                .collect();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
