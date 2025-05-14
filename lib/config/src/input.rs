use multiemu_input::{Input, gamepad::GamepadInput, keyboard::KeyboardInput};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, sync::LazyLock};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
/// Possible hotkeys this emulator could use
pub enum Hotkey {
    ToggleMenu,
    FastForward,
    LoadSnapshot,
    SaveSnapshot,
}

/// Default hotkeys for the application
pub static DEFAULT_HOTKEYS: LazyLock<BTreeMap<BTreeSet<Input>, Hotkey>> = LazyLock::new(|| {
    [
        (
            [
                Input::Gamepad(GamepadInput::Mode),
                Input::Gamepad(GamepadInput::Start),
            ]
            .into(),
            Hotkey::ToggleMenu,
        ),
        (
            [Input::Keyboard(KeyboardInput::F1)].into(),
            Hotkey::ToggleMenu,
        ),
        (
            [
                Input::Gamepad(GamepadInput::Mode),
                Input::Gamepad(GamepadInput::Select),
            ]
            .into(),
            Hotkey::FastForward,
        ),
        (
            [Input::Keyboard(KeyboardInput::F2)].into(),
            Hotkey::FastForward,
        ),
        (
            [
                Input::Gamepad(GamepadInput::Mode),
                Input::Gamepad(GamepadInput::FPadUp),
            ]
            .into(),
            Hotkey::SaveSnapshot,
        ),
        (
            [Input::Keyboard(KeyboardInput::F3)].into(),
            Hotkey::SaveSnapshot,
        ),
        (
            [
                Input::Gamepad(GamepadInput::Mode),
                Input::Gamepad(GamepadInput::FPadLeft),
            ]
            .into(),
            Hotkey::LoadSnapshot,
        ),
        (
            [Input::Keyboard(KeyboardInput::F4)].into(),
            Hotkey::LoadSnapshot,
        ),
    ]
    .into()
});
