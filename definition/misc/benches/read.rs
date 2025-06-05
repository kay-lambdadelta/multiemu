use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_rom::{manager::RomManager, system::GameSystem};
use multiemu_runtime::{builder::MachineBuilder, display::backend::software::SoftwareRendering};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use std::{hint::black_box, sync::Arc};

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_runtime::utils::set_main_thread();

    let rom_manager = Arc::new(RomManager::new(None, None).unwrap());

    let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
        GameSystem::Unknown,
        rom_manager,
        Ratio::from_integer(44100),
    )
    .insert_address_space(64);

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

    let mut buffer = [0; 1];
    c.bench_function("read1", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, cpu_address_space, black_box(&mut buffer))
                .unwrap();
        })
    });

    let mut buffer = [0; 2];
    c.bench_function("read2", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, cpu_address_space, black_box(&mut buffer))
                .unwrap();
        })
    });

    let mut buffer = [0; 4];
    c.bench_function("read4", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, cpu_address_space, black_box(&mut buffer))
                .unwrap();
        })
    });

    let mut buffer = [0; 8];
    c.bench_function("read8", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, cpu_address_space, black_box(&mut buffer))
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
