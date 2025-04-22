use multiemu_input::{GamepadId, Input, InputState};

mod audio;
mod gamepad;
mod keyboard;
pub mod renderer;
pub mod windowing;

pub enum RuntimeBoundMessage {
    Input {
        id: GamepadId,
        input: Input,
        state: InputState,
    },
}
