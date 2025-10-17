use crate::windowing::RuntimeBoundMessage;
use gilrs::{Axis, Button, EventType, GilrsBuilder};
use multiemu_runtime::input::{GamepadId, Input, InputState, gamepad::GamepadInput};
use std::collections::HashMap;
use uuid::Uuid;
use winit::event_loop::EventLoopProxy;

pub fn gamepad_task(sender: EventLoopProxy<RuntimeBoundMessage>) {
    let mut gilrs_context = GilrsBuilder::new().build().unwrap();
    let mut non_stable_controller_identification = HashMap::new();

    loop {
        while let Some(ev) = gilrs_context.next_event_blocking(None) {
            let mut gamepad_id = Uuid::from_bytes(gilrs_context.gamepad(ev.id).uuid());

            if gamepad_id == Uuid::nil() {
                tracing::warn!(
                    "Gamepad {} is not giving us an ID, assigning it a arbitary one",
                    ev.id
                );

                gamepad_id = *non_stable_controller_identification
                    .entry(ev.id)
                    .or_insert_with(Uuid::new_v4);
            }

            let gamepad_id = GamepadId::new(gamepad_id);

            match ev.event {
                EventType::AxisChanged(axis, value, _) => {
                    if let Some((input, state)) = gilrs_axis2input(axis, value)
                        && sender
                            .send_event(RuntimeBoundMessage::Input {
                                id: gamepad_id,
                                input,
                                state,
                            })
                            .is_err()
                    {
                        return;
                    }
                }
                EventType::ButtonChanged(button, value, _) => {
                    if let Some(input) = gilrs_button2input(button)
                        && sender
                            .send_event(RuntimeBoundMessage::Input {
                                id: gamepad_id,
                                input,
                                state: InputState::Analog(value),
                            })
                            .is_err()
                    {
                        return;
                    }
                }
                EventType::Disconnected => {
                    non_stable_controller_identification.remove(&ev.id);
                }
                _ => {}
            }
        }
    }
}

fn gilrs_button2input(button: Button) -> Option<Input> {
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

fn gilrs_axis2input(axis: Axis, value: f32) -> Option<(Input, InputState)> {
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
