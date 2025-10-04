use criterion::{Criterion, criterion_group, criterion_main};
use multiemu::{component::Component, machine::Machine, rom::RomId};
use multiemu_definition_misc::memory::rom::{RomMemory, RomMemoryConfig};
use std::{hint::black_box, str::FromStr};

fn criterion_benchmark(c: &mut Criterion) {
    multiemu::utils::set_main_thread();

    let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, memory) = machine.insert_component(
        "memory",
        RomMemoryConfig {
            rom: RomId::from_str("6e6e37ec8d66aea1c13ed444863e3db91497aa35").unwrap(),
            assigned_range: 0x0000..=0x1fff,
            assigned_address_space: cpu_address_space,
            rom_range: 0x0000..=0x1fff,
        },
    );

    let machine = machine.build(Default::default());
    let component_id = machine.component_registry.get_id(&memory).unwrap();

    c.bench_function("read1", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u8>(0x0fff, cpu_address_space)
                    .unwrap(),
            );
        })
    });

    c.bench_function("read2", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u16>(0x0fff, cpu_address_space)
                    .unwrap(),
            );
        })
    });

    c.bench_function("read4", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u32>(0x0fff, cpu_address_space)
                    .unwrap(),
            );
        })
    });

    c.bench_function("read8", |b| {
        b.iter(|| {
            black_box(
                machine
                    .memory_access_table
                    .read_le_value::<u64>(0x0fff, cpu_address_space)
                    .unwrap(),
            );
        })
    });

    // Please do not skip the access table in actual memory code
    let mut buffer = [0; 1];
    c.bench_function("read1_access_table_skip", |b| {
        b.iter(|| {
            machine
                .component_registry
                .interact::<RomMemory, _>(component_id, |component| {
                    component
                        .read_memory(0x0fff, cpu_address_space, &mut buffer)
                        .unwrap();
                })
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
