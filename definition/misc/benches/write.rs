use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_runtime::machine::Machine;
use rangemap::RangeInclusiveMap;
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
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

    let machine = machine.build(());

    c.bench_function("write1", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write_le_value::<u8>(0x1000, cpu_address_space, black_box(0))
                .unwrap();
        })
    });

    c.bench_function("write2", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write_le_value::<u16>(0x1000, cpu_address_space, black_box(0))
                .unwrap();
        })
    });

    c.bench_function("write4", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write_le_value::<u32>(0x1000, cpu_address_space, black_box(0))
                .unwrap();
        })
    });

    c.bench_function("write8", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write_le_value::<u64>(0x1000, cpu_address_space, black_box(0))
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
