use itertools::Itertools;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display, str::FromStr};

// TODO: The parser could be more rigorous

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
/// Error for invalid component names
pub enum Error {
    #[error("Component path component contains whitespace")]
    /// Component names cannot contain whitespaces
    Whitespace,
    #[error("Component path is too short")]
    /// Component names cannot be empty
    TooShort,
    #[error("Component path cannot hold this character")]
    InvalidCharacter(char),
}

/// Component paths are names seperated by `/`
///
/// Valid formats include
///
/// "`component_1`"
///
/// "`component_1/component_2`"
///
/// "`component_1/component_2/component_3`/"
///
/// Component names cannot contain whitespace and their segments cannot be empty

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentPath(Cow<'static, str>);

impl Value for ComponentPath {
    type SelfType<'a> = Self;
    type AsBytes<'a> = String;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        String::from_utf8_lossy(data).parse().unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        value.to_string()
    }

    fn type_name() -> TypeName {
        TypeName::new("component_path")
    }
}

impl Key for ComponentPath {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        String::from_utf8_lossy(data1).cmp(&String::from_utf8_lossy(data2))
    }
}

impl ComponentPath {
    /// Path separator
    pub const SEPARATOR: char = '/';

    /// Checks if a string is a valid component name
    fn validate_path_component(string: &str) -> Option<Error> {
        if string.chars().any(char::is_whitespace) {
            return Some(Error::Whitespace);
        }

        if string.is_empty() {
            return Some(Error::TooShort);
        }

        if string.contains('.') {
            return Some(Error::InvalidCharacter('.'));
        }

        if string.contains(':') {
            return Some(Error::InvalidCharacter(':'));
        }

        None
    }

    /// Get the parent component, if any
    #[must_use]
    pub fn parent(&self) -> Option<ComponentPath> {
        let mut segments: Vec<&str> = self.iter().collect();

        if segments.len() <= 1 {
            return None;
        }
        segments.pop();

        Some(ComponentPath(Cow::Owned(segments.join("/"))))
    }

    /// Iter over path components
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.split(Self::SEPARATOR).filter(|s| !s.is_empty())
    }

    /// Get the number of path components that exist
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Is it empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check for overlapping
    pub fn contains(&self, other: &ComponentPath) -> bool {
        if self.0 == other.0 {
            return true;
        }

        self.iter().zip(other.iter()).all(|(a, b)| a == b) && self.len() >= other.len()
    }

    /// Push a new component onto this path
    pub fn push(&mut self, segment: &str) -> Result<(), Error> {
        if let Some(err) = Self::validate_path_component(segment) {
            return Err(err);
        }

        let owned = self.0.to_mut();
        owned.push(Self::SEPARATOR);
        owned.push_str(segment);

        Ok(())
    }
}

impl Display for ComponentPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.iter().join("/"))
    }
}

impl FromStr for ComponentPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let segments: Vec<&str> = s
            .split(Self::SEPARATOR)
            .filter(|seg| !seg.is_empty())
            .collect();

        if segments.is_empty() {
            return Err(Error::TooShort);
        }

        for seg in &segments {
            if let Some(err) = Self::validate_path_component(seg) {
                return Err(err);
            }
        }

        Ok(ComponentPath(Cow::Owned(segments.join("/"))))
    }
}

impl Serialize for ComponentPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ComponentPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
/// Path denoting a resource, like an audio output or a display
pub struct ResourcePath {
    /// Component that owns this resource
    pub component: ComponentPath,
    /// Unique resource name
    pub name: Cow<'static, str>,
}

impl Display for ResourcePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}:{}", self.component, self.name))
    }
}

impl FromStr for ResourcePath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ':');
        let path_str = parts.next().ok_or(Error::TooShort)?;
        let name_str = parts.next().ok_or(Error::TooShort)?;

        let component_path = path_str.parse::<ComponentPath>()?;
        if name_str.is_empty() {
            return Err(Error::TooShort);
        }
        if let Some(err) = ComponentPath::validate_path_component(name_str) {
            return Err(err);
        }

        Ok(ResourcePath {
            component: component_path,
            name: Cow::Owned(name_str.to_string()),
        })
    }
}

impl Serialize for ResourcePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ResourcePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
