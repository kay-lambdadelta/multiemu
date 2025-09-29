use crate::{Input, gamepad::GamepadInput, keyboard::KeyboardInput};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter)]
/// Possible hotkeys this emulator could use
pub enum Hotkey {
    ToggleMenu,
    FastForward,
    LoadSnapshot,
    StoreSnapshot,
    IncrementSnapshotCounter,
    DecrementSnapshotCounter,
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
                Input::Gamepad(GamepadInput::DPadLeft),
            ]
            .into(),
            Hotkey::StoreSnapshot,
        ),
        (
            [Input::Keyboard(KeyboardInput::F3)].into(),
            Hotkey::StoreSnapshot,
        ),
        (
            [
                Input::Gamepad(GamepadInput::Mode),
                Input::Gamepad(GamepadInput::DPadRight),
            ]
            .into(),
            Hotkey::LoadSnapshot,
        ),
        (
            [Input::Keyboard(KeyboardInput::F4)].into(),
            Hotkey::LoadSnapshot,
        ),
        (
            [
                Input::Gamepad(GamepadInput::Mode),
                Input::Gamepad(GamepadInput::DPadUp),
            ]
            .into(),
            Hotkey::IncrementSnapshotCounter,
        ),
        (
            [Input::Keyboard(KeyboardInput::F5)].into(),
            Hotkey::IncrementSnapshotCounter,
        ),
        (
            [
                Input::Gamepad(GamepadInput::Mode),
                Input::Gamepad(GamepadInput::DPadDown),
            ]
            .into(),
            Hotkey::DecrementSnapshotCounter,
        ),
        (
            [Input::Keyboard(KeyboardInput::F6)].into(),
            Hotkey::DecrementSnapshotCounter,
        ),
    ]
    .into()
});
