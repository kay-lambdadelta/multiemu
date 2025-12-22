use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use fluxemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use fluxemu_runtime::machine::Machine;
use rangemap::RangeInclusiveMap;

fn criterion_benchmark(c: &mut Criterion) {
    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "memory",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x0000..=0xffff,
            assigned_address_space: address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0x0000..=0xffff,
                StandardMemoryInitialContents::Value(0x00),
            )]),
            sram: false,
        },
    );

    let machine = machine.build(());
    let address_space = machine.address_spaces(address_space).unwrap();
    let mut address_space_cache = address_space.cache();

    c.bench_function("read1", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u8>(0x1000, machine.now(), None)
                    .unwrap(),
            );
        })
    });
    c.bench_function("read2", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u16>(0x1000, machine.now(), None)
                    .unwrap(),
            );
        })
    });
    c.bench_function("read4", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u32>(0x1000, machine.now(), None)
                    .unwrap(),
            );
        })
    });
    c.bench_function("read8", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u64>(0x1000, machine.now(), None)
                    .unwrap(),
            );
        })
    });

    c.bench_function("read1_cached", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u8>(0x1000, machine.now(), Some(&mut address_space_cache))
                    .unwrap(),
            );
        })
    });
    c.bench_function("read2_cached", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u16>(0x1000, machine.now(), Some(&mut address_space_cache))
                    .unwrap(),
            );
        })
    });
    c.bench_function("read4_cached", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u32>(0x1000, machine.now(), Some(&mut address_space_cache))
                    .unwrap(),
            );
        })
    });
    c.bench_function("read8_cached", |b| {
        b.iter(|| {
            black_box(
                address_space
                    .read_le_value::<u64>(0x1000, machine.now(), Some(&mut address_space_cache))
                    .unwrap(),
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
