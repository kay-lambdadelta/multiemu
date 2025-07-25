use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_audio::FrameIterator;
use nalgebra::{Vector1, Vector2, Vector4};
use std::hint::black_box;

const BUFFER_SIZE: usize = 4096;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("mix1_1", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector1::new(0i16); BUFFER_SIZE])
                .into_iter()
                .remix::<1>()
                .collect();
        })
    });

    c.bench_function("upmix1_2", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector1::new(0i16); BUFFER_SIZE])
                .into_iter()
                .remix::<2>()
                .collect();
        })
    });

    c.bench_function("downmix2_1", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .remix::<1>()
                .collect();
        })
    });

    c.bench_function("mix2_2", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .remix::<2>()
                .collect();
        })
    });

    c.bench_function("upmix2_4", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector2::new(0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .remix::<4>()
                .collect();
        })
    });

    c.bench_function("downmix4_2", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector4::new(0i16, 0i16, 0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .remix::<2>()
                .collect();
        })
    });

    c.bench_function("mix4_4", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box([Vector4::new(0i16, 0i16, 0i16, 0i16); BUFFER_SIZE])
                .into_iter()
                .remix::<4>()
                .collect();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
