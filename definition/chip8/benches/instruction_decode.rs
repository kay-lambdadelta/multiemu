use criterion::{Criterion, criterion_group, criterion_main};
use multiemu_config::Environment;
use multiemu_definition_chip8::Chip8InstructionDecoder;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::{
    builder::MachineBuilder, display::software::SoftwareRendering, memory::AddressSpaceId,
    processor::decoder::InstructionDecoder,
};
use multiemu_rom::{manager::RomManager, system::GameSystem};
use std::{
    hint::black_box,
    sync::{Arc, RwLock},
};

const ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

fn criterion_benchmark(c: &mut Criterion) {
    let environment = Arc::new(RwLock::new(Environment::default()));
    let rom_manager = Arc::new(RomManager::new(None).unwrap());

    let machine = MachineBuilder::new(
        GameSystem::Unknown,
        rom_manager.clone(),
        environment.clone(),
    )
    .insert_address_space(ADDRESS_SPACE, 64)
    .insert_component::<StandardMemory>(
        "workram",
        StandardMemoryConfig {
            max_word_size: 8,
            readable: true,
            writable: true,
            assigned_range: 0x0000..=0xffff,
            assigned_address_space: ADDRESS_SPACE,
            initial_contents: vec![StandardMemoryInitialContents::Random],
        },
    )
    .build::<SoftwareRendering>(Default::default());
    let decoder = Chip8InstructionDecoder;

    c.bench_function("decode", |b| {
        b.iter(|| {
            let address = rand::random_range(0..0x5000);
            let _ = decoder.decode(address, black_box(machine.memory_translation_table()));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
