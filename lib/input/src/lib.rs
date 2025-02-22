use gamepad::GamepadInput;
use keyboard::KeyboardInput;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};
use strum::IntoEnumIterator;
use uuid::Uuid;

pub mod gamepad;
pub mod keyboard;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Input {
    Gamepad(GamepadInput),
    Keyboard(KeyboardInput),
}

impl Input {
    pub fn iter() -> impl Iterator<Item = Self> {
        GamepadInput::iter()
            .map(Input::Gamepad)
            .chain(KeyboardInput::iter().map(Input::Keyboard))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InputState {
    /// 0 or 1
    Digital(bool),
    /// Clamped from 0.0 to 1.0
    Analog(f32),
}

impl Default for InputState {
    fn default() -> Self {
        Self::Digital(false)
    }
}

impl InputState {
    pub const PRESSED: Self = Self::Digital(true);
    pub const RELEASED: Self = Self::Digital(false);

    pub fn as_digital(&self, threshhold: Option<f32>) -> bool {
        match self {
            InputState::Digital(value) => *value,
            InputState::Analog(value) => *value >= threshhold.unwrap_or(0.5),
        }
    }

    pub fn as_analog(&self) -> f32 {
        match self {
            InputState::Digital(value) => {
                if *value {
                    1.0
                } else {
                    0.0
                }
            }
            InputState::Analog(value) => *value,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct VirtualGamepadName(Cow<'static, str>);

impl VirtualGamepadName {
    pub const fn new(id: &'static str) -> Self {
        Self(Cow::Borrowed(id))
    }
}

impl AsRef<str> for VirtualGamepadName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for VirtualGamepadName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct GamepadId(Uuid);

impl GamepadId {
    pub const PLATFORM_RESERVED: GamepadId = GamepadId(Uuid::from_u128(0));

    pub const fn new(id: Uuid) -> Self {
        assert!(!id.is_nil(), "Gamepad ID 0 is reserved");

        Self(id)
    }
}

impl Display for GamepadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
