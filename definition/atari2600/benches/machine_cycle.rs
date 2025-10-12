use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_base::{
    environment::{ENVIRONMENT_LOCATION, Environment},
    machine::{Machine, MachineFactory},
    platform::TestPlatform,
    program::{ProgramMetadata, RomId},
};
use multiemu_definition_atari2600::Atari2600;
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
        .identify_program([RomId::from_str("6e6e37ec8d66aea1c13ed444863e3db91497aa35").unwrap()])
        .unwrap()
        .expect("You need a copy of \"Donkey Kong (USA)\" to run this benchmark");

    let machine = Machine::build(
        Some(program_specification),
        program_manager,
        None,
        None,
        Ratio::from_integer(44100),
    );
    let mut machine: Machine<TestPlatform> = Atari2600.construct(machine).build((), false);

    let scheduler_state = machine.scheduler_state.as_mut().unwrap();
    let full_cycle = scheduler_state.timeline_length();

    c.bench_function("atari_2600_full_machine_cycle", |b| {
        b.iter(|| {
            scheduler_state.run_for_cycles(full_cycle);
        })
    });

    let one_second = Duration::from_secs(1);
    c.bench_function("atari_2600_one_second", |b| {
        b.iter(|| {
            scheduler_state.run(one_second);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
