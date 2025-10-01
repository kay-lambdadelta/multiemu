use criterion::{Criterion, criterion_group, criterion_main};
use multiemu::machine::Machine;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use rangemap::RangeInclusiveMap;
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    multiemu::utils::set_main_thread();

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

    c.bench_function("component_ref_read", |b| {
        b.iter(|| {
            component_ref
                .interact(|component| {
                    black_box(component);
                })
                .unwrap();
        })
    });

    c.bench_function("component_ref_write", |b| {
        b.iter(|| {
            component_ref
                .interact_mut(|component| {
                    black_box(component);
                })
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
