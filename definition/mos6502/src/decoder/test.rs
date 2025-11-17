use multiemu_definition_misc::memory::standard::StandardMemoryConfig;
use multiemu_runtime::{machine::Machine, processor::InstructionDecoder};

use crate::{
    Mos6502Kind,
    decoder::Mos6502InstructionDecoder,
    instruction::{
        AddressingMode, Mos6502AddressingMode, Mos6502InstructionSet, Mos6502Opcode, Opcode,
    },
};

#[test]
pub fn decoding_test() {
    let table = [
        (
            0xb6,
            Mos6502InstructionSet {
                opcode: Opcode::Mos6502(Mos6502Opcode::Ldx),
                addressing_mode: Some(AddressingMode::Mos6502(
                    Mos6502AddressingMode::YIndexedZeroPage,
                )),
            },
        ),
        (
            0xbe,
            Mos6502InstructionSet {
                opcode: Opcode::Mos6502(Mos6502Opcode::Ldx),
                addressing_mode: Some(AddressingMode::Mos6502(
                    Mos6502AddressingMode::YIndexedAbsolute,
                )),
            },
        ),
        (
            0x01,
            Mos6502InstructionSet {
                opcode: Opcode::Mos6502(Mos6502Opcode::Ora),
                addressing_mode: Some(AddressingMode::Mos6502(
                    Mos6502AddressingMode::XIndexedZeroPageIndirect,
                )),
            },
        ),
    ];

    let (machine, address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "memory",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x00..=0x00,
            assigned_address_space: address_space,
            initial_contents: Default::default(),
            sram: false,
        },
    );

    let machine = machine.build(());
    let address_space = machine.address_spaces.get(&address_space).unwrap();

    let decoder = Mos6502InstructionDecoder::new(Mos6502Kind::Mos6502);

    for (byte, expected_decoding) in table {
        address_space.write_le_value::<u8>(0x0, None, byte).unwrap();

        let (decoding, _) = decoder.decode(0x0000, address_space, None).unwrap();

        assert_eq!(
            decoding, expected_decoding,
            "Byte 0x{:04x} failed to decode",
            byte
        );
    }
}
