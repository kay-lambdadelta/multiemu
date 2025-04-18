use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_config::Environment;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::{
    builder::MachineBuilder,
    display::{backend::software::SoftwareRendering, shader::ShaderCache},
    memory::AddressSpaceId,
};
use multiemu_rom::{manager::RomManager, system::GameSystem};
use std::{
    hint::black_box,
    sync::{Arc, RwLock},
};

const ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

fn criterion_benchmark(c: &mut Criterion) {
    let environment = Arc::new(RwLock::new(Environment::default()));
    let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
    let shader_cache = Arc::new(ShaderCache::default());

    let machine = MachineBuilder::new(GameSystem::Unknown, rom_manager, environment, shader_cache)
        .insert_address_space(ADDRESS_SPACE, 64)
        .insert_component::<StandardMemory>(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=0xffff,
                assigned_address_space: ADDRESS_SPACE,
                initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
            },
        )
        .build::<SoftwareRendering>(Default::default());

    let mut buffer = [0; 1];
    c.bench_function("read1", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, ADDRESS_SPACE, black_box(&mut buffer))
                .unwrap();
        })
    });

    let mut buffer = [0; 2];
    c.bench_function("read2", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, ADDRESS_SPACE, black_box(&mut buffer))
                .unwrap();
        })
    });

    let mut buffer = [0; 4];
    c.bench_function("read4", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, ADDRESS_SPACE, black_box(&mut buffer))
                .unwrap();
        })
    });

    let mut buffer = [0; 8];
    c.bench_function("read8", |b| {
        b.iter(|| {
            machine
                .memory_translation_table
                .read(0x1000, ADDRESS_SPACE, black_box(&mut buffer))
                .unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
