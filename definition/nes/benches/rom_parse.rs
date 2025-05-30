use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_nes::INes;

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_machine::utils::set_main_thread();

    let rom = std::fs::read("INSERT YOUR FAVORITE NES ROM HERE").unwrap();

    c.bench_function("ines_decode", |b| {
        b.iter(|| {
            INes::parse(&rom).unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
