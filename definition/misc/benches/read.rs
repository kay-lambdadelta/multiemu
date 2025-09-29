use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_runtime::{Machine, component::Component};
use rangemap::RangeInclusiveMap;

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_runtime::utils::set_main_thread();

    let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, component_ref) = machine.insert_component(
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

    let machine = machine.build(Default::default());
    let component_id = component_ref.id();

    let mut buffer = [0; 1];
    c.bench_function("read1", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .read(0x1000, cpu_address_space, &mut buffer)
                .unwrap();
        })
    });

    let mut buffer = [0; 2];
    c.bench_function("read2", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .read(0x1000, cpu_address_space, &mut buffer)
                .unwrap();
        })
    });

    let mut buffer = [0; 4];
    c.bench_function("read4", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .read(0x1000, cpu_address_space, &mut buffer)
                .unwrap();
        })
    });

    let mut buffer = [0; 8];
    c.bench_function("read8", |b| {
        b.iter(|| {
            machine
                .memory_access_table
                .read(0x1000, cpu_address_space, &mut buffer)
                .unwrap();
        })
    });

    // Please do not skip the access table in actual memory code
    let mut buffer = [0; 1];
    c.bench_function("read1_access_table_skip", |b| {
        b.iter(|| {
            machine
                .component_registry
                .interact::<StandardMemory, _>(component_id, |component| {
                    component
                        .read_memory(0x1000, cpu_address_space, &mut buffer)
                        .unwrap();
                })
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
