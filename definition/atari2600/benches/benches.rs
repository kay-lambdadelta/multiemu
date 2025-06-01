use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_config::Environment;
use multiemu_definition_atari2600::Atari2600;
use multiemu_machine::{
    MachineFactory,
    display::{
        backend::software::{SoftwareComponentInitializationData, SoftwareRendering},
        shader::ShaderCache,
    },
};
use multiemu_rom::{id::RomId, manager::RomManager};
use std::{
    hint::black_box,
    str::FromStr,
    sync::{Arc, RwLock},
};

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_machine::utils::set_main_thread();

    let environment = Environment::load().unwrap();
    let rom_manager = Arc::new(
        RomManager::new(
            Some(&environment.database_file),
            Some(&environment.roms_directory),
        )
        .unwrap(),
    );
    let environment = Arc::new(RwLock::new(environment));
    let shader_cache = ShaderCache::new(environment.clone());

    c.bench_function("initialization", |b| {
        b.iter(|| {
            <Atari2600 as MachineFactory<SoftwareRendering>>::construct(
                &Atari2600,
                // Donkey Kong (USA).a26
                vec![RomId::from_str("6e6e37ec8d66aea1c13ed444863e3db91497aa35").unwrap()],
                rom_manager.clone(),
                environment.clone(),
                shader_cache.clone(),
            )
            .build(SoftwareComponentInitializationData)
        })
    });

    let machine = <Atari2600 as MachineFactory<SoftwareRendering>>::construct(
        &Atari2600,
        // Donkey Kong (USA).a26
        vec![RomId::from_str("6e6e37ec8d66aea1c13ed444863e3db91497aa35").unwrap()],
        rom_manager.clone(),
        environment.clone(),
        shader_cache.clone(),
    )
    .build(SoftwareComponentInitializationData);

    let cpu_address_space = machine
        .memory_translation_table
        .address_spaces()
        .next()
        .unwrap();

    c.bench_function("riot_ram_access", |b| {
        b.iter(|| {
            let _: u8 = black_box(
                machine
                    .memory_translation_table
                    .read_le_value(black_box(0x180), cpu_address_space)
                    .unwrap(),
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
