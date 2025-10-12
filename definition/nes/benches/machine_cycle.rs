use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_base::{
    environment::{ENVIRONMENT_LOCATION, Environment},
    machine::{Machine, MachineFactory},
    platform::TestPlatform,
    program::{ProgramMetadata, RomId},
};
use multiemu_definition_nes::Nes;
use num::rational::Ratio;
use std::{
    fs::File,
    ops::Deref,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

fn criterion_benchmark(c: &mut Criterion) {
    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

    let program_manager =
        Arc::new(ProgramMetadata::new(Arc::new(RwLock::new(environment))).unwrap());
    let program_specification = program_manager
        .identify_program([RomId::from_str("3737eefc3c7b1934e929b551251cf1ea98f5f451").unwrap()])
        .unwrap()
        .expect("You need a copy of \"BurgerTime (USA)\" to run this benchmark");

    let machine = Machine::build(
        Some(program_specification),
        program_manager,
        None,
        None,
        Ratio::from_integer(44100),
    );
    let mut machine: Machine<TestPlatform> = Nes.construct(machine).build((), false);
    let scheduler_state = machine.scheduler_state.as_mut().unwrap();
    let full_cycle = scheduler_state.timeline_length();

    c.bench_function("nes_full_machine_cycle", |b| {
        b.iter(|| {
            scheduler_state.run_for_cycles(full_cycle);
        })
    });

    let one_second = Duration::from_secs(1);
    c.bench_function("nes_one_second", |b| {
        b.iter(|| {
            scheduler_state.run(one_second);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
