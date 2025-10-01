#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SingleByteArgument {
    Register(Register),
    HlIndirect,
}

impl SingleByteArgument {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0b000 => Some(SingleByteArgument::Register(Register::B)),
            0b001 => Some(SingleByteArgument::Register(Register::C)),
            0b010 => Some(SingleByteArgument::Register(Register::D)),
            0b011 => Some(SingleByteArgument::Register(Register::E)),
            0b100 => Some(SingleByteArgument::Register(Register::H)),
            0b101 => Some(SingleByteArgument::Register(Register::L)),
            0b110 => Some(SingleByteArgument::HlIndirect),
            0b111 => Some(SingleByteArgument::Register(Register::A)),
            _ => None,
        }
    }
}

pub enum I8080Instruction {
    Nop,
    Ld,
}

pub enum Lr35902Instruction {}

pub enum Z80Instruction {}

pub enum InstructionSet {
    I8080(I8080Instruction),
    Lr35902(Lr35902Instruction),
    Z80(Z80Instruction),
}
