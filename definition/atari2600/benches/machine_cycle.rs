use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_config::{ENVIRONMENT_LOCATION, Environment};
use multiemu_definition_atari2600::Atari2600;
use multiemu_rom::{RomId, RomMetadata};
use multiemu_runtime::{
    Machine, MachineFactory, UserSpecifiedRoms, platform::TestPlatform,
    utils::DirectMainThreadExecutor,
};
use num::rational::Ratio;
use std::{fs::File, ops::Deref, str::FromStr, sync::Arc};

fn criterion_benchmark(c: &mut Criterion) {
    multiemu_runtime::utils::set_main_thread();
    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

    let rom_manager = Arc::new(
        RomMetadata::new(
            Some(environment.database_location.0.clone()),
            Some(environment.rom_store_directory.0.clone()),
        )
        .unwrap(),
    );
    let machine = Machine::build(
        Some(
            UserSpecifiedRoms::from_id(
                RomId::from_str("6e6e37ec8d66aea1c13ed444863e3db91497aa35").unwrap(),
                &rom_manager,
            )
            .unwrap(),
        ),
        rom_manager,
        None,
        None,
        Ratio::from_integer(44100),
        Arc::new(DirectMainThreadExecutor),
    );
    let machine: Machine<TestPlatform> = Atari2600.construct(machine).build(Default::default());

    let mut scheduler_guard = machine.scheduler.lock().unwrap();
    let full_cycle = scheduler_guard.full_cycle();

    c.bench_function("full_machine_cycle", |b| {
        b.iter(|| {
            scheduler_guard.run_for_cycles(full_cycle);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
