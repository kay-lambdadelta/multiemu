use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_config::{ENVIRONMENT_LOCATION, Environment};
use multiemu_definition_atari2600::Atari2600;
use multiemu_rom::{RomId, RomManager};
use multiemu_runtime::{MachineFactory, platform::TestPlatform, utils::DirectMainThreadExecutor};
use num::rational::Ratio;
use std::{fs::File, hint::black_box, ops::Deref, str::FromStr, sync::Arc};

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_runtime::utils::set_main_thread();

    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

    let rom_manager = Arc::new(
        RomManager::new(
            Some(environment.database_location.0.clone()),
            Some(environment.rom_store_directory.0.clone()),
        )
        .unwrap(),
    );

    let machine = MachineFactory::<TestPlatform>::construct(
        &Atari2600,
        // Donkey Kong (USA).a26
        vec![RomId::from_str("6e6e37ec8d66aea1c13ed444863e3db91497aa35").unwrap()],
        rom_manager.clone(),
        Ratio::from_integer(44100),
        Arc::new(DirectMainThreadExecutor),
    )
    .build(Default::default());

    let cpu_address_space = machine
        .memory_access_table
        .address_spaces()
        .next()
        .unwrap();

    c.bench_function("riot_ram_access", |b| {
        b.iter(|| {
            let _: u8 = black_box(
                machine
                    .memory_access_table
                    .read_le_value(black_box(0x180), cpu_address_space)
                    .unwrap(),
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
