use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_runtime::Machine;
use rangemap::RangeInclusiveMap;
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_runtime::utils::set_main_thread();

    let (mut machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

    for i in (0x0000..=0x1000).step_by(0x1f) {
        machine = machine
            .insert_component(
                &format!("memory_{}", i),
                StandardMemoryConfig {
                    readable: true,
                    writable: true,
                    assigned_range: i..=i + 0x1f,
                    assigned_address_space: cpu_address_space,
                    initial_contents: RangeInclusiveMap::from_iter([(
                        i..=i + 0x1f,
                        StandardMemoryInitialContents::Value(0x00),
                    )]),
                    sram: false,
                },
            )
            .0;
    }

    let machine = machine.build(Default::default());

    let buffer = [0; 1];
    c.bench_function("write1", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });

    let buffer = [0; 2];
    c.bench_function("write2", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });

    let buffer = [0; 4];
    c.bench_function("write4", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });

    let buffer = [0; 8];
    c.bench_function("write8", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
