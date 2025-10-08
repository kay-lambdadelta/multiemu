use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use multiemu::machine::Machine;
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use rangemap::RangeInclusiveMap;

fn criterion_benchmark(c: &mut Criterion) {
    multiemu::utils::set_main_thread();

    let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "memory",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x0000..=0xffff,
            assigned_address_space: cpu_address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0x0000..=0xffff,
                StandardMemoryInitialContents::Value(0x00),
            )]),
            sram: false,
        },
    );

    let machine = machine.build(Default::default(), false);

    c.bench_function("read1", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u8>(0x1000, cpu_address_space)
                    .unwrap(),
            );
        })
    });

    c.bench_function("read2", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u16>(0x1000, cpu_address_space)
                    .unwrap(),
            );
        })
    });

    c.bench_function("read4", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u32>(0x1000, cpu_address_space)
                    .unwrap(),
            );
        })
    });

    c.bench_function("read8", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u64>(0x1000, cpu_address_space)
                    .unwrap(),
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
