use multiemu_machine::{
    memory::{AddressSpaceId, memory_translation_table::MemoryTranslationTable},
    processor::instruction::{InstructionSet, InstructionTextRepresentation},
};
use std::borrow::Cow;

// https://www.pagetable.com/c64ref/6502/?tab=2

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AddressingMode {
    Accumulator,
    Immediate(u8),
    Absolute(u16),
    XIndexedAbsolute(u16),
    YIndexedAbsolute(u16),
    AbsoluteIndirect(u16),
    ZeroPage(u8),
    XIndexedZeroPage(u8),
    YIndexedZeroPage(u8),
    ZeroPageYIndexed(u8),
    XIndexedZeroPageIndirect(u8),
    ZeroPageIndirectYIndexed(u8),
    Relative(i8),
}

impl AddressingMode {
    #[inline]
    pub fn from_group1_addressing(
        cursor: u16,
        address_space: AddressSpaceId,
        memory_translation_table: &MemoryTranslationTable,
        addressing_mode: u8,
    ) -> (Self, u8) {
        match addressing_mode {
            0b000 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::XIndexedZeroPageIndirect(indirect), 1)
            }
            0b001 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPage(indirect), 1)
            }
            0b010 => {
                let mut immediate = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut immediate),
                );

                (AddressingMode::Immediate(immediate), 1)
            }
            0b011 => {
                let mut indirect = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut indirect);

                (AddressingMode::Absolute(u16::from_le_bytes(indirect)), 2)
            }
            0b100 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPageIndirectYIndexed(indirect), 1)
            }
            0b101 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::XIndexedZeroPage(indirect), 1)
            }
            0b110 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::YIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            0b111 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::XIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            _ => {
                unreachable!()
            }
        }
    }

    #[inline]
    pub fn from_group2_addressing(
        cursor: u16,
        address_space: AddressSpaceId,
        memory_translation_table: &MemoryTranslationTable,
        addressing_mode: u8,
    ) -> (Self, u8) {
        match addressing_mode {
            0b000 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::XIndexedZeroPageIndirect(indirect), 1)
            }
            0b001 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPage(indirect), 1)
            }
            0b010 => (AddressingMode::Accumulator, 0),
            0b011 => {
                let mut indirect = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut indirect);

                (AddressingMode::Absolute(u16::from_le_bytes(indirect)), 2)
            }
            0b100 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::ZeroPageIndirectYIndexed(indirect), 1)
            }
            0b101 => {
                let mut indirect = 0;
                let _ = memory_translation_table.read(
                    cursor as usize,
                    address_space,
                    std::array::from_mut(&mut indirect),
                );

                (AddressingMode::XIndexedZeroPage(indirect), 1)
            }
            0b110 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::YIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            0b111 => {
                let mut absolute = [0; 2];
                let _ =
                    memory_translation_table.read(cursor as usize, address_space, &mut absolute);

                (
                    AddressingMode::XIndexedAbsolute(u16::from_le_bytes(absolute)),
                    2,
                )
            }
            _ => {
                unreachable!()
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum M6502InstructionSetSpecifier {
    Adc,
    Anc,
    And,
    Arr,
    Asl,
    Asr,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dcp,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Isc,
    Jam,
    Jmp,
    Jsr,
    Las,
    Lax,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rla,
    Rol,
    Ror,
    Rra,
    Rti,
    Rts,
    Sax,
    Sbc,
    Sbx,
    Sec,
    Sed,
    Sei,
    Sha,
    Shs,
    Shx,
    Shy,
    Slo,
    Sre,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
    Xaa,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct M6502InstructionSet {
    pub specifier: M6502InstructionSetSpecifier,
    pub addressing_mode: Option<AddressingMode>,
}

impl InstructionSet for M6502InstructionSet {
    fn to_text_representation(&self) -> InstructionTextRepresentation {
        InstructionTextRepresentation {
            instruction_mnemonic: Cow::Borrowed(match self.specifier {
                M6502InstructionSetSpecifier::Adc => "ADC",
                M6502InstructionSetSpecifier::Anc => "ANC",
                M6502InstructionSetSpecifier::And => "AND",
                M6502InstructionSetSpecifier::Arr => "ARR",
                M6502InstructionSetSpecifier::Asl => "ASL",
                M6502InstructionSetSpecifier::Asr => "ASR",
                M6502InstructionSetSpecifier::Bcc => "BCC",
                M6502InstructionSetSpecifier::Bcs => "BCS",
                M6502InstructionSetSpecifier::Beq => "BEQ",
                M6502InstructionSetSpecifier::Bit => "BIT",
                M6502InstructionSetSpecifier::Bmi => "BMI",
                M6502InstructionSetSpecifier::Bne => "BNE",
                M6502InstructionSetSpecifier::Bpl => "BPL",
                M6502InstructionSetSpecifier::Brk => "BRK",
                M6502InstructionSetSpecifier::Bvc => "BVC",
                M6502InstructionSetSpecifier::Bvs => "BVS",
                M6502InstructionSetSpecifier::Clc => "CLC",
                M6502InstructionSetSpecifier::Cld => "CLD",
                M6502InstructionSetSpecifier::Cli => "CLI",
                M6502InstructionSetSpecifier::Clv => "CLV",
                M6502InstructionSetSpecifier::Cmp => "CMP",
                M6502InstructionSetSpecifier::Cpx => "CPX",
                M6502InstructionSetSpecifier::Cpy => "CPY",
                M6502InstructionSetSpecifier::Dcp => "DCP",
                M6502InstructionSetSpecifier::Dec => "DEC",
                M6502InstructionSetSpecifier::Dex => "DEX",
                M6502InstructionSetSpecifier::Dey => "DEY",
                M6502InstructionSetSpecifier::Eor => "EOR",
                M6502InstructionSetSpecifier::Inc => "INC",
                M6502InstructionSetSpecifier::Inx => "INX",
                M6502InstructionSetSpecifier::Iny => "INY",
                M6502InstructionSetSpecifier::Isc => "ISC",
                M6502InstructionSetSpecifier::Jam => "JAM",
                M6502InstructionSetSpecifier::Jmp => "JMP",
                M6502InstructionSetSpecifier::Jsr => "JSR",
                M6502InstructionSetSpecifier::Las => "LAS",
                M6502InstructionSetSpecifier::Lax => "LAX",
                M6502InstructionSetSpecifier::Lda => "LDA",
                M6502InstructionSetSpecifier::Ldx => "LDX",
                M6502InstructionSetSpecifier::Ldy => "LDY",
                M6502InstructionSetSpecifier::Lsr => "LSR",
                M6502InstructionSetSpecifier::Nop => "NOP",
                M6502InstructionSetSpecifier::Ora => "ORA",
                M6502InstructionSetSpecifier::Pha => "PHA",
                M6502InstructionSetSpecifier::Php => "PHP",
                M6502InstructionSetSpecifier::Pla => "PLA",
                M6502InstructionSetSpecifier::Plp => "PLP",
                M6502InstructionSetSpecifier::Rla => "RLA",
                M6502InstructionSetSpecifier::Rol => "ROL",
                M6502InstructionSetSpecifier::Ror => "ROR",
                M6502InstructionSetSpecifier::Rra => "RRA",
                M6502InstructionSetSpecifier::Rti => "RTI",
                M6502InstructionSetSpecifier::Rts => "RTS",
                M6502InstructionSetSpecifier::Sax => "SAX",
                M6502InstructionSetSpecifier::Sbc => "SBC",
                M6502InstructionSetSpecifier::Sbx => "SBX",
                M6502InstructionSetSpecifier::Sec => "SEC",
                M6502InstructionSetSpecifier::Sed => "SED",
                M6502InstructionSetSpecifier::Sei => "SEI",
                M6502InstructionSetSpecifier::Sha => "SHA",
                M6502InstructionSetSpecifier::Shs => "SHS",
                M6502InstructionSetSpecifier::Shx => "SHX",
                M6502InstructionSetSpecifier::Shy => "SHY",
                M6502InstructionSetSpecifier::Slo => "SLO",
                M6502InstructionSetSpecifier::Sre => "SRE",
                M6502InstructionSetSpecifier::Sta => "STA",
                M6502InstructionSetSpecifier::Stx => "STX",
                M6502InstructionSetSpecifier::Sty => "STY",
                M6502InstructionSetSpecifier::Tax => "TAX",
                M6502InstructionSetSpecifier::Tay => "TAY",
                M6502InstructionSetSpecifier::Tsx => "TSX",
                M6502InstructionSetSpecifier::Txa => "TXA",
                M6502InstructionSetSpecifier::Txs => "TXS",
                M6502InstructionSetSpecifier::Tya => "TYA",
                M6502InstructionSetSpecifier::Xaa => "XAA",
            }),
        }
    }
}
