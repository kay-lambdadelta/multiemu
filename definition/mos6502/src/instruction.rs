use multiemu_runtime::processor::InstructionSet;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::Mos6502Kind;

// https://www.pagetable.com/c64ref/6502/?tab=2

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mos6502AddressingMode {
    Immediate,
    Absolute,
    XIndexedAbsolute,
    YIndexedAbsolute,
    AbsoluteIndirect,
    ZeroPage,
    XIndexedZeroPage,
    YIndexedZeroPage,
    XIndexedZeroPageIndirect,
    ZeroPageIndirectYIndexed,
    Relative,
    Accumulator,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wdc65C02AddressingMode {
    ZeroPageIndirect,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressingMode {
    Mos6502(Mos6502AddressingMode),
    Wdc65C02(Wdc65C02AddressingMode),
}

impl AddressingMode {
    pub fn is_valid_for_mode(&self, mode: Mos6502Kind) -> bool {
        match mode {
            Mos6502Kind::Mos6502 => matches!(self, AddressingMode::Mos6502(_)),
            Mos6502Kind::Mos6507 => matches!(self, AddressingMode::Mos6502(_)),
            Mos6502Kind::Ricoh2A0x => matches!(self, AddressingMode::Mos6502(_)),
            Mos6502Kind::Wdc65C02 => matches!(
                self,
                AddressingMode::Mos6502(_) | AddressingMode::Wdc65C02(_)
            ),
        }
    }
}

impl AddressingMode {
    pub fn added_instruction_length(&self) -> u16 {
        match self {
            AddressingMode::Mos6502(Mos6502AddressingMode::Immediate) => 1,
            AddressingMode::Mos6502(Mos6502AddressingMode::Absolute) => 2,
            AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedAbsolute) => 2,
            AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedAbsolute) => 2,
            AddressingMode::Mos6502(Mos6502AddressingMode::AbsoluteIndirect) => 2,
            AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPage) => 1,
            AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPage) => 1,
            AddressingMode::Mos6502(Mos6502AddressingMode::YIndexedZeroPage) => 1,
            AddressingMode::Mos6502(Mos6502AddressingMode::XIndexedZeroPageIndirect) => 1,
            AddressingMode::Mos6502(Mos6502AddressingMode::ZeroPageIndirectYIndexed) => 1,
            AddressingMode::Mos6502(Mos6502AddressingMode::Relative) => 1,
            AddressingMode::Mos6502(Mos6502AddressingMode::Accumulator) => 0,
            AddressingMode::Wdc65C02(Wdc65C02AddressingMode::ZeroPageIndirect) => 1,
        }
    }
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, strum::Display,
)]
pub enum Mos6502Opcode {
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

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, strum::Display,
)]
pub enum Wdc65C02Opcode {
    Bra,
    Phx,
    Phy,
    Plx,
    Ply,
    Stz,
    Trb,
    Tsb,
    // Apparently these two only exist on some 65C02Os but for simplicity sake we will treat all of them as having these two
    Stp,
    Wai,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Opcode {
    Mos6502(Mos6502Opcode),
    Wdc65C02(Wdc65C02Opcode),
}

impl Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Opcode::Mos6502(opcode) => write!(f, "{}", opcode),
            Opcode::Wdc65C02(opcode) => write!(f, "{}", opcode),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mos6502InstructionSet {
    pub opcode: Opcode,
    pub addressing_mode: Option<AddressingMode>,
}

impl Mos6502InstructionSet {
    pub fn is_addressing_mode_valid(&self, kind: Mos6502Kind) -> bool {
        match self.opcode {
            Opcode::Mos6502(opcode) => match opcode {
                Mos6502Opcode::Adc => todo!(),
                Mos6502Opcode::Anc => todo!(),
                Mos6502Opcode::And => todo!(),
                Mos6502Opcode::Arr => todo!(),
                Mos6502Opcode::Asl => todo!(),
                Mos6502Opcode::Asr => todo!(),
                Mos6502Opcode::Bcc => todo!(),
                Mos6502Opcode::Bcs => todo!(),
                Mos6502Opcode::Beq => todo!(),
                Mos6502Opcode::Bit => todo!(),
                Mos6502Opcode::Bmi => todo!(),
                Mos6502Opcode::Bne => todo!(),
                Mos6502Opcode::Bpl => todo!(),
                Mos6502Opcode::Brk => todo!(),
                Mos6502Opcode::Bvc => todo!(),
                Mos6502Opcode::Bvs => todo!(),
                Mos6502Opcode::Clc => todo!(),
                Mos6502Opcode::Cld => todo!(),
                Mos6502Opcode::Cli => todo!(),
                Mos6502Opcode::Clv => todo!(),
                Mos6502Opcode::Cmp => todo!(),
                Mos6502Opcode::Cpx => todo!(),
                Mos6502Opcode::Cpy => todo!(),
                Mos6502Opcode::Dcp => todo!(),
                Mos6502Opcode::Dec => todo!(),
                Mos6502Opcode::Dex => todo!(),
                Mos6502Opcode::Dey => todo!(),
                Mos6502Opcode::Eor => todo!(),
                Mos6502Opcode::Inc => todo!(),
                Mos6502Opcode::Inx => todo!(),
                Mos6502Opcode::Iny => todo!(),
                Mos6502Opcode::Isc => todo!(),
                Mos6502Opcode::Jam => todo!(),
                Mos6502Opcode::Jmp => todo!(),
                Mos6502Opcode::Jsr => todo!(),
                Mos6502Opcode::Las => todo!(),
                Mos6502Opcode::Lax => todo!(),
                Mos6502Opcode::Lda => todo!(),
                Mos6502Opcode::Ldx => todo!(),
                Mos6502Opcode::Ldy => todo!(),
                Mos6502Opcode::Lsr => todo!(),
                Mos6502Opcode::Nop => todo!(),
                Mos6502Opcode::Ora => todo!(),
                Mos6502Opcode::Pha => todo!(),
                Mos6502Opcode::Php => todo!(),
                Mos6502Opcode::Pla => todo!(),
                Mos6502Opcode::Plp => todo!(),
                Mos6502Opcode::Rla => todo!(),
                Mos6502Opcode::Rol => todo!(),
                Mos6502Opcode::Ror => todo!(),
                Mos6502Opcode::Rra => todo!(),
                Mos6502Opcode::Rti => todo!(),
                Mos6502Opcode::Rts => todo!(),
                Mos6502Opcode::Sax => todo!(),
                Mos6502Opcode::Sbc => todo!(),
                Mos6502Opcode::Sbx => todo!(),
                Mos6502Opcode::Sec => todo!(),
                Mos6502Opcode::Sed => todo!(),
                Mos6502Opcode::Sei => todo!(),
                Mos6502Opcode::Sha => todo!(),
                Mos6502Opcode::Shs => todo!(),
                Mos6502Opcode::Shx => todo!(),
                Mos6502Opcode::Shy => todo!(),
                Mos6502Opcode::Slo => todo!(),
                Mos6502Opcode::Sre => todo!(),
                Mos6502Opcode::Sta => todo!(),
                Mos6502Opcode::Stx => todo!(),
                Mos6502Opcode::Sty => todo!(),
                Mos6502Opcode::Tax => todo!(),
                Mos6502Opcode::Tay => todo!(),
                Mos6502Opcode::Tsx => todo!(),
                Mos6502Opcode::Txa => todo!(),
                Mos6502Opcode::Txs => todo!(),
                Mos6502Opcode::Tya => todo!(),
                Mos6502Opcode::Xaa => todo!(),
            },
            Opcode::Wdc65C02(opcode) => match opcode {
                Wdc65C02Opcode::Bra => todo!(),
                Wdc65C02Opcode::Phx => todo!(),
                Wdc65C02Opcode::Phy => todo!(),
                Wdc65C02Opcode::Plx => todo!(),
                Wdc65C02Opcode::Ply => todo!(),
                Wdc65C02Opcode::Stz => todo!(),
                Wdc65C02Opcode::Trb => todo!(),
                Wdc65C02Opcode::Tsb => todo!(),
                Wdc65C02Opcode::Stp => todo!(),
                Wdc65C02Opcode::Wai => todo!(),
            },
        }
    }
}

impl InstructionSet for Mos6502InstructionSet {
    type Opcode = Opcode;
    type AddressingMode = AddressingMode;
}
