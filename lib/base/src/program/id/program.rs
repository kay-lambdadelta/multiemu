use crate::program::MachineId;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramId {
    pub machine: MachineId,
    pub name: String,
}

impl Display for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.machine, self.name)
    }
}

impl FromStr for ProgramId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Find the first '[' and last ']' to separate system and name
        let start_bracket = s
            .find('[')
            .ok_or_else(|| format!("Missing '[' in '{}'", s))?;
        let end_bracket = s
            .rfind(']')
            .ok_or_else(|| format!("Missing ']' in '{}'", s))?;

        if start_bracket >= end_bracket {
            return Err(format!("Invalid bracket positions in '{}'", s));
        }

        let system_str = &s[..start_bracket];
        let name = &s[start_bracket + 1..end_bracket];

        if name.is_empty() {
            return Err(format!("Program name is empty in '{}'", s));
        }

        let system = MachineId::from_str(system_str)
            .map_err(|_| format!("Invalid system string '{}'", system_str))?;

        Ok(ProgramId {
            machine: system,
            name: name.to_string(),
        })
    }
}

impl Value for ProgramId {
    type SelfType<'a> = Self;

    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::serde::decode_from_slice(data, bincode::config::standard())
            .unwrap()
            .0
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        bincode::serde::encode_to_vec(value, bincode::config::standard()).unwrap()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("program_id")
    }
}

impl Key for ProgramId {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        data1.cmp(data2)
    }
}
