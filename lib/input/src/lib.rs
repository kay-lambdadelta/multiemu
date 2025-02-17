use gamepad::GamepadInput;
use keyboard::KeyboardInput;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

pub mod gamepad;
pub mod keyboard;
pub mod virtual_gamepad;

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

/// Id of a gamepad currently recognized by the emulator
pub type GamepadId = u8;
