use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
/// Error for invalid component names
pub enum ComponentNameError {
    #[error("Component name contains whitespace")]
    /// Component names cannot contain whitespaces
    Whitespace,
    #[error("Component name is too long")]
    /// Component names cannot be longer than 100 characters
    TooLong,
    #[error("Component name is too short")]
    /// Component names cannot be empty
    TooShort,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A component name, works as its on disk identifier
pub struct ComponentName(String);

impl ComponentName {
    /// Maximum length of a component name, in bytes
    pub const MAX_NAME_LENGTH: usize = 100;

    /// Checks if a string is a valid component name
    fn validity_checks(string: &str) -> Option<ComponentNameError> {
        if string.chars().any(|c| c.is_whitespace()) {
            return Some(ComponentNameError::Whitespace);
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
