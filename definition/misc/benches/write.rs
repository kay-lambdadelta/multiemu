use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_config::Environment;
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::{
    builder::MachineBuilder,
    display::{backend::software::SoftwareRendering, shader::ShaderCache},
};
use multiemu_rom::{manager::RomManager, system::GameSystem};
use rangemap::RangeInclusiveMap;
use std::{
    hint::black_box,
    sync::{Arc, RwLock},
};

fn criterion_benchmark(c: &mut Criterion) {
    unsafe {
        multiemu_machine::utils::set_main_thread();
    }

    let environment = Arc::new(RwLock::new(Environment::default()));
    let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
    let shader_cache = ShaderCache::new(environment.clone());

    let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
        GameSystem::Unknown,
        rom_manager,
        environment,
        shader_cache,
    )
    .insert_address_space("cpu", 64);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            max_word_size: 8,
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
