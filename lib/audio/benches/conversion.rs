use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_audio::SampleIterator;
use std::hint::black_box;

const BUFFER_SIZE: usize = 4096;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("u8_to_i8", |b| {
        b.iter(|| {
            let _: Vec<i8> = black_box([0u8; BUFFER_SIZE])
                .into_iter()
                .rescale()
                .collect();
        })
    });

    c.bench_function("i8_to_u8", |b| {
        b.iter(|| {
            let _: Vec<u8> = black_box([0i8; BUFFER_SIZE])
                .into_iter()
                .rescale()
                .collect();
        })
    });

    c.bench_function("u8_to_i16", |b| {
        b.iter(|| {
            let _: Vec<i16> = black_box([0u8; BUFFER_SIZE])
                .into_iter()
                .rescale()
                .collect();
        })
    });

    c.bench_function("i16_to_u8", |b| {
        b.iter(|| {
            let _: Vec<u8> = black_box([0i16; BUFFER_SIZE])
                .into_iter()
                .rescale()
                .collect();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
