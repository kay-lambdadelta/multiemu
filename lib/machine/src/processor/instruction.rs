use std::fmt::Debug;
use std::{borrow::Cow, fmt::Display};

#[derive(Debug)]
pub struct InstructionTextRepresentation {
    pub instruction_mnemonic: Cow<'static, str>,
}

impl Display for InstructionTextRepresentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.instruction_mnemonic)
    }
}

pub trait InstructionSet: Debug + Eq + Clone + Send + Sync + 'static {
    fn to_text_representation(&self) -> InstructionTextRepresentation;
}
