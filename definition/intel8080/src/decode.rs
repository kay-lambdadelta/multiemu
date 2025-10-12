use super::instruction::SingleByteArgument;
use bitvec::field::BitField;
use bitvec::prelude::Msb0;
use bitvec::view::BitView;
use multiemu_base::memory::memory_access_table::MemoryAccessTable;
use std::ops::Range;

const INSTRUCTION_IDENTIFIER: Range<usize> = 0..2;
const SECONDARY_INSTRUCTION_IDENTIFIER: Range<usize> = 5..8;
const ARGUMENT: Range<usize> = 2..5;

pub fn decode_instruction(
    cursor: usize,
    memory_access_table: &MemoryAccessTable,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut instruction_first_byte = 0;
    memory_access_table.read(cursor, std::slice::from_mut(&mut instruction_first_byte));
    let instruction_first_byte = instruction_first_byte.view_bits::<Msb0>();
    let instruction_identifier = instruction_first_byte[INSTRUCTION_IDENTIFIER].load::<u8>();

    match instruction_identifier {
        0b00 => {
            todo!()
        }
        0b01 => {
            let source_register = instruction_first_byte[ARGUMENT].load::<u8>();
            let destination_register =
                instruction_first_byte[SECONDARY_INSTRUCTION_IDENTIFIER].load::<u8>();

            let source_register = SingleByteArgument::from_id(source_register).unwrap();
            let destination_register = SingleByteArgument::from_id(destination_register).unwrap();

            if source_register == SingleByteArgument::HlIndirect
                && destination_register == SingleByteArgument::HlIndirect
            {}
        }
        0b10 => {
            todo!()
        }
        0b11 => {
            todo!()
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}
