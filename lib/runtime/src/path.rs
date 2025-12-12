use itertools::Itertools;
use redb::{Key, TypeName, Value};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Namespace {
    Component,
    Resource,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
/// Error for invalid component names
pub enum Error {
    #[error("Path segment contains whitespace")]
    /// Component names cannot contain whitespaces
    Whitespace,
    #[error("Path is too short")]
    /// Component names cannot be empty
    TooShort,
    #[error("Segment cannot hold this character")]
    InvalidCharacter(char),
    #[error("Component path cannot hold this character")]
    InvalidPathType(String),
}

/// Valid formats include
///
/// "`:component/component_1`"
///
/// "`:component/component_1/component_2`"
///
/// "`:resource/component_1/component_2/resource_1/`"
///
/// Item names cannot be empty or contain whitespace or /
///
/// Resources cannot have sub items, but can be top level items.

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MultiemuPath(String);

impl Value for MultiemuPath {
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
        TypeName::new("path")
    }
}

impl Key for MultiemuPath {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        String::from_utf8_lossy(data1).cmp(&String::from_utf8_lossy(data2))
    }
}

impl MultiemuPath {
    /// Path separator
    pub const SEPARATOR: char = '/';

    pub fn namespace(&self) -> Namespace {
        match self.0.split(Self::SEPARATOR).next().unwrap() {
            ":resource" => Namespace::Resource,
            ":component" => Namespace::Component,
            _ => {
                unreachable!()
            }
        }
    }

    pub fn push(&mut self, namespace: Namespace, segment: &str) {
        assert_ne!(self.namespace(), Namespace::Resource);

        let new_base = self.0.replace(
            ":component",
            match namespace {
                Namespace::Component => ":component",
                Namespace::Resource => ":resource",
            },
        );

        let segment = "/".to_string() + segment;
        self.0 = new_base + &segment;
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.split(Self::SEPARATOR).skip(1)
    }

    pub fn parent(&self) -> Option<MultiemuPath> {
        let segment_count = self.iter().count();

        if segment_count < 1 {
            return None;
        }

        let path = self.iter().take(segment_count - 1).join("/");

        Some(MultiemuPath(format!(":component/{}", path)))
    }
}

impl Display for MultiemuPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for MultiemuPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let segments: Vec<&str> = s
            .split(Self::SEPARATOR)
            .filter(|seg| !seg.is_empty())
            .collect();

        if segments.len() < 2 {
            return Err(Error::TooShort);
        }

        match segments[0] {
            ":resource" => Namespace::Resource,
            ":component" => Namespace::Component,
            _ => {
                return Err(Error::InvalidPathType(segments[0].to_string()));
            }
        };

        for segment in &segments[1..] {
            if segment.contains(char::is_whitespace) {
                return Err(Error::Whitespace);
            }
        }

        Ok(MultiemuPath(s.to_string()))
    }
}

impl Serialize for MultiemuPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

impl<'de> Deserialize<'de> for MultiemuPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl AsRef<str> for MultiemuPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
