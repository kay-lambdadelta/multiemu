use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_base::{
    environment::{ENVIRONMENT_LOCATION, Environment},
    machine::{Machine, MachineFactory},
    platform::TestPlatform,
    program::{ProgramMetadata, RomId},
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
    multiemu_base::utils::set_main_thread();
    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

    let program_manager =
        Arc::new(ProgramMetadata::new(Arc::new(RwLock::new(environment))).unwrap());
    let program_specification = program_manager
        .identify_program([
            RomId::from_str("3737eefc3c7b1934e929b551251cf1ea98f5f451").unwrap(),
        ])
        .unwrap()
        .expect("You need a copy of \"BurgerTime (USA)\" to run this benchmark");

    c.bench_function("nes_machine_initialization", |b| {
        b.iter(|| {
            let machine = Machine::build(
                Some(program_specification.clone()),
                program_manager.clone(),
                None,
                None,
                Ratio::from_integer(44100),
                Arc::new(DirectMainThreadExecutor),
            );

            let _: Machine<TestPlatform> = Nes.construct(machine).build((), false);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
