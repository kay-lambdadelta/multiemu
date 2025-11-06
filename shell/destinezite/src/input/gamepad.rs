use gilrs::{Axis, Button};
use multiemu_runtime::input::{GamepadInput, Input, InputState};

pub fn gilrs_button2input(button: Button) -> Option<Input> {
    Some(Input::Gamepad(match button {
        Button::South => GamepadInput::FPadDown,
        Button::East => GamepadInput::FPadRight,
        Button::North => GamepadInput::FPadUp,
        Button::West => GamepadInput::FPadLeft,
        Button::C => todo!(),
        Button::Z => GamepadInput::ZTrigger,
        Button::LeftTrigger => GamepadInput::LeftTrigger,
        Button::LeftTrigger2 => GamepadInput::LeftSecondaryTrigger,
        Button::RightTrigger => GamepadInput::RightTrigger,
        Button::RightTrigger2 => GamepadInput::RightSecondaryTrigger,
        Button::Select => GamepadInput::Select,
        Button::Start => GamepadInput::Start,
        Button::Mode => GamepadInput::Mode,
        Button::LeftThumb => GamepadInput::LeftThumb,
        Button::RightThumb => GamepadInput::RightThumb,
        Button::DPadUp => GamepadInput::DPadUp,
        Button::DPadDown => GamepadInput::DPadDown,
        Button::DPadLeft => GamepadInput::DPadLeft,
        Button::DPadRight => GamepadInput::DPadRight,
        Button::Unknown => return None,
    }))
}

pub fn gilrs_axis2input(axis: Axis, value: f32) -> Option<(Input, InputState)> {
    match axis {
        Axis::LeftStickX => Some((
            Input::Gamepad(if value < 0.0 {
                GamepadInput::LeftStickLeft
            } else {
                GamepadInput::LeftStickRight
            }),
            InputState::Analog(value.abs().clamp(0.0, 1.0)),
        )),
        Axis::LeftStickY => Some((
            Input::Gamepad(if value < 0.0 {
                GamepadInput::LeftStickDown
            } else {
                GamepadInput::LeftStickUp
            }),
            InputState::Analog(value.abs().clamp(0.0, 1.0)),
        )),
        Axis::RightStickX => Some((
            Input::Gamepad(if value < 0.0 {
                GamepadInput::RightStickLeft
            } else {
                GamepadInput::RightStickRight
            }),
            InputState::Analog(value.abs().clamp(0.0, 1.0)),
        )),
        Axis::RightStickY => Some((
            Input::Gamepad(if value < 0.0 {
                GamepadInput::RightStickDown
            } else {
                GamepadInput::RightStickUp
            }),
            InputState::Analog(value.abs().clamp(0.0, 1.0)),
        )),
        // Needs investigation what this actually means
        Axis::LeftZ => todo!(),
        Axis::RightZ => todo!(),
        Axis::DPadX => Some((
            Input::Gamepad(if value < 0.0 {
                GamepadInput::DPadLeft
            } else {
                GamepadInput::DPadRight
            }),
            InputState::Analog(value.abs().clamp(0.0, 1.0)),
        )),
        Axis::DPadY => Some((
            Input::Gamepad(if value < 0.0 {
                GamepadInput::DPadUp
            } else {
                GamepadInput::DPadDown
            }),
            InputState::Analog(value.abs().clamp(0.0, 1.0)),
        )),
        Axis::Unknown => None,
    }
}
