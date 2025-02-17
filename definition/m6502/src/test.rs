use super::instruction::{AddressingMode, M6502InstructionSet, M6502InstructionSetSpecifier};
use crate::decode::decode_instruction;
use multiemu_config::Environment;
use multiemu_definition_misc::memory::standard::{
    StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
};
use multiemu_machine::display::software::SoftwareRendering;
use multiemu_machine::memory::AddressSpaceId;
use multiemu_machine::Machine;
use multiemu_rom::{manager::RomManager, system::GameSystem};
use std::hash::RandomState;
use std::sync::{Arc, RwLock};
use std::{borrow::Cow, collections::HashMap};

const ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

#[test]
fn m6502_instruction_decode() {
    let environment = Arc::new(RwLock::new(Environment::load().unwrap()));
    let rom_manager = Arc::new(RomManager::new(None).unwrap());

    let map: HashMap<_, _, RandomState> = HashMap::from_iter([
        (
            [0x00].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Brk,
                    addressing_mode: None,
                },
                1,
            ),
        ),
        (
            [0x01, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(AddressingMode::Immediate(0xff)),
                },
                2,
            ),
        ),
        (
            [0x02].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Jam,
                    addressing_mode: None,
                },
                1,
            ),
        ),
        (
            [0x03, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Slo,
                    addressing_mode: Some(AddressingMode::XIndexedZeroPageIndirect(0xff)),
                },
                2,
            ),
        ),
        (
            [0x04, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Nop,
                    addressing_mode: Some(AddressingMode::ZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x05, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(AddressingMode::ZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x06, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Asl,
                    addressing_mode: Some(AddressingMode::ZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x07, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Slo,
                    addressing_mode: Some(AddressingMode::ZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x08].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Php,
                    addressing_mode: None,
                },
                1,
            ),
        ),
        (
            [0x09, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(AddressingMode::Immediate(0xff)),
                },
                2,
            ),
        ),
        (
            [0x0a].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Asl,
                    addressing_mode: Some(AddressingMode::Accumulator),
                },
                1,
            ),
        ),
        (
            [0x0b, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Anc,
                    addressing_mode: Some(AddressingMode::Immediate(0xff)),
                },
                2,
            ),
        ),
        (
            [0x0c, 0xff, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Nop,
                    addressing_mode: Some(AddressingMode::Absolute(0xffff)),
                },
                3,
            ),
        ),
        (
            [0x0d, 0xff, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(AddressingMode::Absolute(0xffff)),
                },
                3,
            ),
        ),
        (
            [0x0e, 0xff, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Asl,
                    addressing_mode: Some(AddressingMode::Absolute(0xffff)),
                },
                3,
            ),
        ),
        (
            [0x0f, 0xff, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Slo,
                    addressing_mode: Some(AddressingMode::Absolute(0xffff)),
                },
                3,
            ),
        ),
        (
            [0x10, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Bpl,
                    addressing_mode: Some(AddressingMode::Relative(-1)),
                },
                2,
            ),
        ),
        (
            [0x11, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(AddressingMode::ZeroPageIndirectYIndexed(0xff)),
                },
                2,
            ),
        ),
        (
            [0x12].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Jam,
                    addressing_mode: None,
                },
                1,
            ),
        ),
        (
            [0x13, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Slo,
                    addressing_mode: Some(AddressingMode::ZeroPageIndirectYIndexed(0xff)),
                },
                2,
            ),
        ),
        (
            [0x14, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Nop,
                    addressing_mode: Some(AddressingMode::XIndexedZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x15, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(AddressingMode::XIndexedZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x16, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Asl,
                    addressing_mode: Some(AddressingMode::XIndexedZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x17, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Slo,
                    addressing_mode: Some(AddressingMode::XIndexedZeroPage(0xff)),
                },
                2,
            ),
        ),
        (
            [0x18].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Clc,
                    addressing_mode: None,
                },
                1,
            ),
        ),
        (
            [0x19, 0xff, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Ora,
                    addressing_mode: Some(AddressingMode::YIndexedAbsolute(0xffff)),
                },
                3,
            ),
        ),
        (
            [0x1a].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Nop,
                    addressing_mode: None,
                },
                1,
            ),
        ),
        (
            [0x1b, 0xff, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Slo,
                    addressing_mode: Some(AddressingMode::YIndexedAbsolute(0xffff)),
                },
                3,
            ),
        ),
        (
            [0x1c, 0xff, 0xff].as_slice(),
            (
                M6502InstructionSet {
                    specifier: M6502InstructionSetSpecifier::Nop,
                    addressing_mode: Some(AddressingMode::XIndexedAbsolute(0xffff)),
                },
                3,
            ),
        ),
    ]);

    for (instruction_binary, (decoded_instruction, decoded_instruction_size)) in map {
        let machine = Machine::build(
            GameSystem::Unknown,
            rom_manager.clone(),
            environment.clone(),
        )
        .insert_component::<StandardMemory>(StandardMemoryConfig {
            max_word_size: 8,
            readable: true,
            writable: true,
            assigned_range: 0..0x4,
            assigned_address_space: ADDRESS_SPACE,
            initial_contents: StandardMemoryInitialContents::Array {
                value: Cow::Borrowed(instruction_binary),
                offset: 0,
            },
        })
        .0
        .build::<SoftwareRendering>(Default::default());

        let (decoded_instruction_result, decoded_instruction_result_size) =
            decode_instruction(0x0, ADDRESS_SPACE, &machine.memory_translation_table()).unwrap();

        assert_eq!(
            (decoded_instruction, decoded_instruction_size),
            (decoded_instruction_result, decoded_instruction_result_size)
        );
    }
}
