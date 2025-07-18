use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ComponentNameError {
    #[error("Component name contains whitespace")]
    Whitespace,
    #[error("Component name contains banned characters")]
    BadCharacter,
    #[error("Component name is too long")]
    TooLong,
    #[error("Component name is too short")]
    TooShort,
}

pub const BANNED_CHARACTERS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '\''];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentName(String);

impl ComponentName {
    pub const MAX_NAME_LENGTH: usize = 100;

    fn validity_checks(string: &str) -> Option<ComponentNameError> {
        if string.chars().any(|c| c.is_whitespace()) {
            return Some(ComponentNameError::Whitespace);
        }

        if string.chars().any(|c| BANNED_CHARACTERS.contains(&c)) {
            return Some(ComponentNameError::BadCharacter);
        }

        if string.is_empty() {
            return Some(ComponentNameError::TooShort);
        }

        if string.len() > Self::MAX_NAME_LENGTH {
            return Some(ComponentNameError::TooLong);
        }

        None
    }
}

impl Serialize for ComponentName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ComponentName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ComponentName::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl FromStr for ComponentName {
    type Err = ComponentNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::validity_checks(s) {
            Some(reason) => Err(reason),
            None => Ok(Self(s.to_string())),
        }
    }
}

impl AsRef<str> for ComponentName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
