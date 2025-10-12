use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_base::audio::SampleIterator;
use std::hint::black_box;

const BUFFER_SIZE: usize = 44100;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("u8_to_i8", |b| {
        b.iter(|| {
            black_box([0u8; BUFFER_SIZE])
                .into_iter()
                .rescale::<i8>()
                .for_each(|sample| {
                    black_box(sample);
                });
        })
    });

    c.bench_function("i8_to_u8", |b| {
        b.iter(|| {
            black_box([0i8; BUFFER_SIZE])
                .into_iter()
                .rescale::<u8>()
                .for_each(|sample| {
                    black_box(sample);
                });
        })
    });

    c.bench_function("u8_to_i16", |b| {
        b.iter(|| {
            black_box([0u8; BUFFER_SIZE])
                .into_iter()
                .rescale::<i16>()
                .for_each(|sample| {
                    black_box(sample);
                });
        })
    });

    c.bench_function("i16_to_u8", |b| {
        b.iter(|| {
            black_box([0i16; BUFFER_SIZE])
                .into_iter()
                .rescale::<u8>()
                .for_each(|sample| {
                    black_box(sample);
                });
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
