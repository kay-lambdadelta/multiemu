use criterion::{Criterion, criterion_group, criterion_main};
use multiemu::{
    environment::{ENVIRONMENT_LOCATION, Environment},
    machine::{Machine, MachineFactory, UserSpecifiedRoms},
    platform::TestPlatform,
    rom::{RomId, RomMetadata},
    utils::DirectMainThreadExecutor,
};
use multiemu_definition_nes::Nes;
use num::rational::Ratio;
use std::{
    fs::File,
    ops::Deref,
    str::FromStr,
    sync::{Arc, RwLock},
};

fn criterion_benchmark(c: &mut Criterion) {
    multiemu::utils::set_main_thread();
    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

    let rom_manager = Arc::new(RomMetadata::new(Arc::new(RwLock::new(environment))).unwrap());
    let machine = Machine::build(
        Some(
            UserSpecifiedRoms::from_id(
                // BurgerTime (USA)
                RomId::from_str("3737eefc3c7b1934e929b551251cf1ea98f5f451").unwrap(),
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
    let machine: Machine<TestPlatform> = Nes.construct(machine).build(Default::default());

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
