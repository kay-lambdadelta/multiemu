use crate::{
    Mos6502Kind,
    decoder::Mos6502InstructionDecoder,
    instruction::{
        AddressingMode, Mos6502AddressingMode, Mos6502InstructionSet, Mos6502Opcode, Opcode,
    },
};
use multiemu_definition_misc::memory::standard::StandardMemoryConfig;
use multiemu_runtime::{machine::Machine, processor::InstructionDecoder};

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

    let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

    let (machine, _) = machine.insert_component(
        "memory",
        StandardMemoryConfig {
            readable: true,
            writable: true,
            assigned_range: 0x00..=0x00,
            assigned_address_space: cpu_address_space,
            initial_contents: Default::default(),
            sram: false,
        },
    );

    let machine = machine.build((), false);
    let memory_access_table = machine.memory_access_table;
    let decoder = Mos6502InstructionDecoder::new(Mos6502Kind::Mos6502);

    for (byte, expected_decoding) in table {
        memory_access_table
            .write_le_value::<u8>(0x0, cpu_address_space, byte)
            .unwrap();

        let (decoding, _) = decoder
            .decode(0x0000, cpu_address_space, &memory_access_table)
            .unwrap();

        assert_eq!(
            decoding, expected_decoding,
            "Byte 0x{:04x} failed to decode",
            byte
        );
    }
}
