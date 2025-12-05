use multiemu_runtime::{machine::Machine, scheduler::Period};
use rangemap::RangeInclusiveMap;

use crate::memory::standard::{StandardMemoryConfig, StandardMemoryInitialContents};

#[test]
fn read() {
    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0..=7,
            assigned_address_space: address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0..=7,
                StandardMemoryInitialContents::Value(0xff),
            )]),
            sram: false,
        },
    );
    let machine = machine.build(());
    let address_space = machine.address_spaces(address_space).unwrap();

    let mut buffer = [0; 8];

    address_space
        .read(0, Period::default(), None, &mut buffer)
        .unwrap();
    assert_eq!(buffer, [0xff; 8]);
}

#[test]
fn write() {
    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0..=7,
            assigned_address_space: address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0..=7,
                StandardMemoryInitialContents::Value(0xff),
            )]),
            sram: false,
        },
    );
    let machine = machine.build(());
    let address_space = machine.address_spaces(address_space).unwrap();

    let buffer = [0; 8];

    address_space
        .write(0, Period::default(), None, &buffer)
        .unwrap();
}

#[test]
fn read_write() {
    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0..=7,
            assigned_address_space: address_space,
            initial_contents: RangeInclusiveMap::from_iter([(
                0..=7,
                StandardMemoryInitialContents::Value(0xff),
            )]),
            sram: false,
        },
    );
    let machine = machine.build(());
    let address_space = machine.address_spaces(address_space).unwrap();

    let mut buffer = [0xff; 8];

    address_space
        .write(0, Period::default(), None, &buffer)
        .unwrap();
    buffer.fill(0);
    address_space
        .read(0, Period::default(), None, &mut buffer)
        .unwrap();
    assert_eq!(buffer, [0xff; 8]);
}

#[test]
fn wraparound() {
    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(8);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x00..=0xff,
            assigned_address_space: address_space,
            initial_contents: RangeInclusiveMap::from_iter([
                (0x00..=0x00, StandardMemoryInitialContents::Value(0xff)),
                (0x01..=0xff, StandardMemoryInitialContents::Value(0x00)),
            ]),
            sram: false,
        },
    );
    let machine = machine.build(());
    let address_space = machine.address_spaces(address_space).unwrap();

    let mut buffer = [0; 2];

    address_space
        .read(0xff, Period::default(), None, &mut buffer)
        .unwrap();
    assert_eq!(buffer, [0x00, 0xff]);
}

#[test]
fn mirror() {
    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(8);

    let (machine, _) = machine.insert_component(
        "workram",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x00..=0xff,
            assigned_address_space: address_space,
            initial_contents: RangeInclusiveMap::from_iter([
                (0x00..=0x00, StandardMemoryInitialContents::Value(0xfe)),
                (0x01..=0x01, StandardMemoryInitialContents::Value(0xff)),
            ]),
            sram: false,
        },
    );
    let machine = machine.memory_map_mirror(address_space, 0x02..=0x02, 0x00..=0x00);

    let machine = machine.build(());
    let address_space = machine.address_spaces(address_space).unwrap();

    let mut buffer = [0; 3];

    address_space
        .read(0, Period::default(), None, &mut buffer)
        .unwrap();
    assert_eq!(buffer, [0xfe, 0xff, 0xfe]);
}
