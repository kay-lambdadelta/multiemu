use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_rom::RomManager;
use multiemu_runtime::builder::MachineBuilder;
use rangemap::RangeInclusiveMap;
use std::{hint::black_box, sync::Arc};

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_runtime::utils::set_main_thread();

    let rom_manager = Arc::new(RomManager::new(None, None).unwrap());

    let (machine, cpu_address_space) =
        MachineBuilder::new_test(rom_manager).insert_address_space(64);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0..=0xffff,
            assigned_address_space: cpu_address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0..=0xffff,
                StandardMemoryInitialContents::Value(0x00),
            )]),
        },
    );
    let machine = machine.build(Default::default());

    let buffer = [0; 1];
    c.bench_function("write1", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });

    let buffer = [0; 2];
    c.bench_function("write2", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });

    let buffer = [0; 4];
    c.bench_function("write4", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });

    let buffer = [0; 8];
    c.bench_function("write8", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .write(0x1000, cpu_address_space, black_box(&buffer))
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
