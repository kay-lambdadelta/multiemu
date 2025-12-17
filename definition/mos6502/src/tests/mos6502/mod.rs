use std::sync::Arc;

use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_runtime::{
    machine::Machine, memory::AddressSpaceId, path::MultiemuPath, scheduler::Frequency,
};
use rangemap::RangeInclusiveMap;

use crate::{Mos6502Config, Mos6502Kind};

mod adc;

fn instruction_test_boilerplate() -> (Arc<Machine>, MultiemuPath, AddressSpaceId) {
    let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, cpu) = machine.insert_component(
        "mos6502",
        Mos6502Config {
            frequency: Frequency::ONE,
            assigned_address_space: cpu_address_space,
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
            assigned_address_space: cpu_address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0x0000..=0xffff,
                StandardMemoryInitialContents::Value(0),
            )]),
            sram: false,
        },
    );

    (machine.build(()), cpu, cpu_address_space)
}
