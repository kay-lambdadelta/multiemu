use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_audio::{
    frame::FrameIterator,
    interpolate::{cubic::Cubic, linear::Linear},
};
use nalgebra::Vector2;
use num::rational::Ratio;
use std::hint::black_box;

const BUFFER_SIZE: usize = 4096;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("resample_linear_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Linear::<f32>::default(),
                )
                .collect();
        })
    });

    c.bench_function("resample_linear_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Linear::<f32>::default(),
                )
                .collect();
        })
    });

    c.bench_function("resample_linear_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Linear::<f64>::default(),
                )
                .collect();
        })
    });

    c.bench_function("resample_linear_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Linear::<f64>::default(),
                )
                .collect();
        })
    });

    c.bench_function("resample_cubic_f32_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Cubic::<f32>::default(),
                )
                .collect();
        })
    });

    c.bench_function("resample_cubic_f32_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Cubic::<f32>::default(),
                )
                .collect();
        })
    });

    c.bench_function("resample_cubic_f64_down", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(44100),
                    Ratio::from_integer(440),
                    Cubic::<f64>::default(),
                )
                .collect();
        })
    });

    c.bench_function("resample_cubic_f64_up", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .resample(
                    Ratio::from_integer(440),
                    Ratio::from_integer(44100),
                    Cubic::<f64>::default(),
                )
                .collect();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
