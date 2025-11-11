use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_definition_chip8::Chip8InstructionDecoder;
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_runtime::{machine::Machine, processor::InstructionDecoder};
use rangemap::RangeInclusiveMap;
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x0000..=0xffff,
            assigned_address_space: address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0x0000..=0xffff,
                StandardMemoryInitialContents::Random,
            )]),
            sram: false,
        },
    );
    let machine = machine.build(());
    let address_space = machine.address_spaces.get(&address_space).unwrap();

    let decoder = Chip8InstructionDecoder;

    c.bench_function("decode", |b| {
        b.iter(|| {
            let address = rand::random_range(0..0x5000);
            let _ = decoder.decode(black_box(address), black_box(address_space));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
