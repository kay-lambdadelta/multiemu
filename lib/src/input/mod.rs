use gamepad::GamepadInput;
use keyboard::KeyboardInput;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum::IntoEnumIterator;
use uuid::Uuid;

pub mod gamepad;
pub mod hotkey;
pub mod keyboard;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Enum covering all possible input types
pub enum Input {
    Gamepad(GamepadInput),
    Keyboard(KeyboardInput),
}

impl Input {
    /// Iterate over every possible input
    pub fn iter() -> impl Iterator<Item = Self> {
        GamepadInput::iter()
            .map(Input::Gamepad)
            .chain(KeyboardInput::iter().map(Input::Keyboard))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// Represents the state as collected of a single input
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
    /// Digital press
    pub const PRESSED: Self = Self::Digital(true);

    /// Digital release
    pub const RELEASED: Self = Self::Digital(false);

    /// Interprets self as a digital input
    pub fn as_digital(&self, threshhold: Option<f32>) -> bool {
        match self {
            InputState::Digital(value) => *value,
            InputState::Analog(value) => *value >= threshhold.unwrap_or(0.5),
        }
    }

    /// Interprets self as an analog input
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

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
/// The ID of a real, physical gamepad
pub struct GamepadId(Uuid);

impl GamepadId {
    /// The ID of the platforms default input device
    ///
    /// For desktop, this is the keyboard
    ///
    /// For 3ds/psp this is the built in gamepad
    pub const PLATFORM_RESERVED: GamepadId = GamepadId(Uuid::from_u128(0));

    /// Creates a new gamepad ID
    pub const fn new(id: Uuid) -> Self {
        assert!(!id.is_nil(), "Gamepad ID 0 is reserved");

        Self(id)
    }
}

impl Display for GamepadId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}
