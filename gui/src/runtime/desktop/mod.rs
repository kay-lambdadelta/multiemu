use multiemu_input::{GamepadId, Input, InputState};

mod audio;
mod input;
pub mod renderer;
pub mod windowing;

pub enum RuntimeBoundMessage {
    Input {
        id: GamepadId,
        input: Input,
        state: InputState,
    },
}
