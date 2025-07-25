use crate::{Mos6502, Mos6502Config, Mos6502Kind};
use multiemu_definition_misc::memory::standard::{
    StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_runtime::{
    Machine, builder::MachineBuilder, component::ComponentRef, memory::AddressSpaceHandle,
    platform::TestPlatform,
};
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;

mod adc;

fn instruction_test_boilerplate() -> (
    Machine<TestPlatform>,
    ComponentRef<Mos6502>,
    AddressSpaceHandle,
) {
    let (machine, cpu_address_space) = MachineBuilder::new_test_minimal().insert_address_space(16);

    let (machine, cpu) = machine.insert_component(
        "mos6502",
        Mos6502Config {
            frequency: Ratio::from_integer(1000),
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
