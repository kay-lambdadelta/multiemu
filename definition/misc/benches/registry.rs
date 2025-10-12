use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_base::machine::Machine;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use rangemap::RangeInclusiveMap;
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, memory) = machine.insert_component(
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
    let machine = machine.build((), false);
    let component_id = machine.component_registry.get_id(&memory).unwrap();

    c.bench_function("registry_read", |b| {
        b.iter(|| {
            machine
                .component_registry
                .interact::<StandardMemory, _>(component_id, |component| {
                    black_box(component);
                })
                .unwrap();
        })
    });

    c.bench_function("registry_write", |b| {
        b.iter(|| {
            machine
                .component_registry
                .interact_mut::<StandardMemory, _>(component_id, |component| {
                    black_box(component);
                })
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
