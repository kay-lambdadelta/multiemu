use std::{fs::File, hint::black_box, ops::Deref, str::FromStr};

use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_mos6502::Mos6502;
use multiemu_definition_nes::Nes;
use multiemu_frontend::environment::{ENVIRONMENT_LOCATION, Environment};
use multiemu_runtime::{
    machine::{Machine, MachineFactory},
    platform::TestPlatform,
    program::{ProgramManager, RomId},
};
use num::rational::Ratio;

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

    let machine = Machine::build(
        Some(program_specification),
        program_manager,
        None,
        None,
        Ratio::from_integer(44100),
    );
    let machine: Machine<TestPlatform> = Nes.construct(machine).build(());

    let processor = machine
        .component_registry
        .get::<Mos6502>(&"processor".parse().unwrap())
        .unwrap();
    let address_space = machine
        .address_spaces
        .get(&processor.interact(|c| c.address_space()))
        .unwrap();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
