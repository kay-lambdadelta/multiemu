use std::{fs::File, ops::Deref, str::FromStr, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use fluxemu_definition_nes::Nes;
use fluxemu_frontend::environment::{ENVIRONMENT_LOCATION, Environment};
use fluxemu_runtime::{
    machine::{Machine, MachineFactory, builder::MachineBuilder},
    platform::TestPlatform,
    program::{ProgramManager, RomId},
};

fn criterion_benchmark(c: &mut Criterion) {
    let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
    let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

    let program_manager = ProgramManager::new(
        &environment.database_location,
        &environment.rom_store_directory,
    )
    .unwrap();
    let program_specification = program_manager
        .identify_program([RomId::from_str("3737eefc3c7b1934e929b551251cf1ea98f5f451").unwrap()])
        .unwrap()
        .expect("You need a copy of \"BurgerTime (USA)\" to run this benchmark");

    let machine: MachineBuilder<TestPlatform> =
        Machine::build(Some(program_specification), program_manager, None, None);
    let machine = Nes.construct(machine).build(());

    let one_second = Duration::from_secs(1);
    c.bench_function("nes_one_second", |b| {
        b.iter(|| {
            machine.run_duration(one_second);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
