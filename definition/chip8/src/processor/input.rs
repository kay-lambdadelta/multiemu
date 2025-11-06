use multiemu_runtime::input::{Input, keyboard::KeyboardInput};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub(super) struct Chip8KeyCode(pub u8);

impl TryFrom<Input> for Chip8KeyCode {
    type Error = ();

    fn try_from(value: Input) -> Result<Self, Self::Error> {
        match value {
            Input::Keyboard(KeyboardInput::Numpad0) => Ok(Chip8KeyCode(0x0)),
            Input::Keyboard(KeyboardInput::Numpad1) => Ok(Chip8KeyCode(0x1)),
            Input::Keyboard(KeyboardInput::Numpad2) => Ok(Chip8KeyCode(0x2)),
            Input::Keyboard(KeyboardInput::Numpad3) => Ok(Chip8KeyCode(0x3)),
            Input::Keyboard(KeyboardInput::Numpad4) => Ok(Chip8KeyCode(0x4)),
            Input::Keyboard(KeyboardInput::Numpad5) => Ok(Chip8KeyCode(0x5)),
            Input::Keyboard(KeyboardInput::Numpad6) => Ok(Chip8KeyCode(0x6)),
            Input::Keyboard(KeyboardInput::Numpad7) => Ok(Chip8KeyCode(0x7)),
            Input::Keyboard(KeyboardInput::Numpad8) => Ok(Chip8KeyCode(0x8)),
            Input::Keyboard(KeyboardInput::Numpad9) => Ok(Chip8KeyCode(0x9)),
            Input::Keyboard(KeyboardInput::KeyA) => Ok(Chip8KeyCode(0xa)),
            Input::Keyboard(KeyboardInput::KeyB) => Ok(Chip8KeyCode(0xb)),
            Input::Keyboard(KeyboardInput::KeyC) => Ok(Chip8KeyCode(0xc)),
            Input::Keyboard(KeyboardInput::KeyD) => Ok(Chip8KeyCode(0xd)),
            Input::Keyboard(KeyboardInput::KeyE) => Ok(Chip8KeyCode(0xe)),
            Input::Keyboard(KeyboardInput::KeyF) => Ok(Chip8KeyCode(0xf)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Chip8KeyCode> for Input {
    type Error = ();

    fn try_from(value: Chip8KeyCode) -> Result<Self, Self::Error> {
        match value.0 {
            0x0 => Ok(Input::Keyboard(KeyboardInput::Numpad0)),
            0x1 => Ok(Input::Keyboard(KeyboardInput::Numpad1)),
            0x2 => Ok(Input::Keyboard(KeyboardInput::Numpad2)),
            0x3 => Ok(Input::Keyboard(KeyboardInput::Numpad3)),
            0x4 => Ok(Input::Keyboard(KeyboardInput::Numpad4)),
            0x5 => Ok(Input::Keyboard(KeyboardInput::Numpad5)),
            0x6 => Ok(Input::Keyboard(KeyboardInput::Numpad6)),
            0x7 => Ok(Input::Keyboard(KeyboardInput::Numpad7)),
            0x8 => Ok(Input::Keyboard(KeyboardInput::Numpad8)),
            0x9 => Ok(Input::Keyboard(KeyboardInput::Numpad9)),
            0xa => Ok(Input::Keyboard(KeyboardInput::KeyA)),
            0xb => Ok(Input::Keyboard(KeyboardInput::KeyB)),
            0xc => Ok(Input::Keyboard(KeyboardInput::KeyC)),
            0xd => Ok(Input::Keyboard(KeyboardInput::KeyD)),
            0xe => Ok(Input::Keyboard(KeyboardInput::KeyE)),
            0xf => Ok(Input::Keyboard(KeyboardInput::KeyF)),
            _ => Err(()),
        }
    }
}

pub(super) fn default_bindings() -> HashMap<Input, Input> {
    HashMap::from_iter([
        // Keyboard mappings
        (
            Input::Keyboard(KeyboardInput::Digit1),
            Input::Keyboard(KeyboardInput::Numpad1),
        ),
        (
            Input::Keyboard(KeyboardInput::Digit2),
            Input::Keyboard(KeyboardInput::Numpad2),
        ),
        (
            Input::Keyboard(KeyboardInput::Digit3),
            Input::Keyboard(KeyboardInput::Numpad3),
        ),
        (
            Input::Keyboard(KeyboardInput::Digit4),
            Input::Keyboard(KeyboardInput::KeyC),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyQ),
            Input::Keyboard(KeyboardInput::Numpad4),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyW),
            Input::Keyboard(KeyboardInput::Numpad5),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyE),
            Input::Keyboard(KeyboardInput::Numpad6),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyR),
            Input::Keyboard(KeyboardInput::KeyD),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyA),
            Input::Keyboard(KeyboardInput::Numpad7),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyS),
            Input::Keyboard(KeyboardInput::Numpad8),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyD),
            Input::Keyboard(KeyboardInput::Numpad9),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyF),
            Input::Keyboard(KeyboardInput::KeyE),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyZ),
            Input::Keyboard(KeyboardInput::KeyA),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyX),
            Input::Keyboard(KeyboardInput::Numpad0),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyC),
            Input::Keyboard(KeyboardInput::KeyB),
        ),
        (
            Input::Keyboard(KeyboardInput::KeyV),
            Input::Keyboard(KeyboardInput::KeyF),
        ),
    ])
}

pub(super) fn present_inputs() -> Vec<Input> {
    Vec::from_iter([
        // Interpreting the numbers on the chip8 keypad as "numpad"
        Input::Keyboard(KeyboardInput::Numpad1),
        Input::Keyboard(KeyboardInput::Numpad2),
        Input::Keyboard(KeyboardInput::Numpad3),
        Input::Keyboard(KeyboardInput::KeyC),
        Input::Keyboard(KeyboardInput::Numpad4),
        Input::Keyboard(KeyboardInput::Numpad5),
        Input::Keyboard(KeyboardInput::Numpad6),
        Input::Keyboard(KeyboardInput::KeyD),
        Input::Keyboard(KeyboardInput::Numpad7),
        Input::Keyboard(KeyboardInput::Numpad8),
        Input::Keyboard(KeyboardInput::Numpad9),
        Input::Keyboard(KeyboardInput::KeyE),
        Input::Keyboard(KeyboardInput::KeyA),
        Input::Keyboard(KeyboardInput::Numpad0),
        Input::Keyboard(KeyboardInput::KeyB),
        Input::Keyboard(KeyboardInput::KeyF),
    ])
}
