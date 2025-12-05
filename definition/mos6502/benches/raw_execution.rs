use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_definition_mos6502::{Mos6502Config, Mos6502Kind, RESET_VECTOR};
use multiemu_runtime::{
    machine::Machine,
    memory::Address,
    scheduler::{Frequency, Period},
};
use rangemap::RangeInclusiveMap;

pub const PROGRAM: [u8; 9] = [
    0xe6, 0x01, // inc 0x01
    0xd0, 0xfc, // bne -4
    0xe6, 0x00, // inc 0x00
    0x4c, 0x00, 0x80, // jmp 0x8000
];

fn criterion_benchmark(c: &mut Criterion) {
    let (machine, cpu_address_space_id) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, cpu) = machine.insert_component(
        "mos6502",
        Mos6502Config {
            frequency: Frequency::from_num(1000000),
            assigned_address_space: cpu_address_space_id,
            kind: Mos6502Kind::Mos6502,
            broken_ror: false,
        },
    );

    let (machine, _) = machine.insert_component(
        "memory",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x0000..=0xffff,
            assigned_address_space: cpu_address_space_id,
            initial_contents: RangeInclusiveMap::from_iter([(
                0x0000..=0xffff,
                StandardMemoryInitialContents::Value(0),
            )]),
            sram: false,
        },
    );

    let machine = machine.build(());
    let cpu = machine.component_handle(&cpu).unwrap();
    let cpu_address_space = machine.address_spaces(cpu_address_space_id).unwrap();

    // Write the program
    cpu_address_space
        .write(0x8000, machine.now(), None, &PROGRAM)
        .unwrap();
    cpu_address_space
        .write_le_value(RESET_VECTOR as Address, machine.now(), None, 0x8000)
        .unwrap();

    let one_second = Period::from_num(1);
    let mut timestamp = Period::from_num(0);

    c.bench_function("mos6502_raw_execution_speed_1sec_1mhz", |b| {
        b.iter(|| {
            timestamp += one_second;

            cpu.interact_mut(timestamp, |_| {});
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
