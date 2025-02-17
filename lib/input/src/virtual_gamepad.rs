use super::{Input, InputState};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Mutex;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct VirtualGamepadId(Cow<'static, str>);

impl VirtualGamepadId {
    pub const fn new(id: &'static str) -> Self {
        Self(Cow::Borrowed(id))
    }
}

impl AsRef<str> for VirtualGamepadId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for VirtualGamepadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct VirtualGamepadMetadata {
    pub present_inputs: HashSet<Input>,
    pub default_bindings: HashMap<Input, Input>,
}

#[derive(Debug)]
pub struct VirtualGamepad {
    pub id: VirtualGamepadId,
    metadata: Cow<'static, VirtualGamepadMetadata>,
    state: Mutex<HashMap<Input, InputState>>,
}

impl VirtualGamepad {
    pub fn new(
        id: VirtualGamepadId,
        medadata: impl Into<Cow<'static, VirtualGamepadMetadata>>,
    ) -> Self {
        Self {
            id,
            metadata: medadata.into(),
            state: Mutex::default(),
        }
    }

    pub fn set(&self, input: Input, state: InputState) {
        let mut gamepad_state = self.state.lock().unwrap();

        if self.metadata.present_inputs.contains(&input) {
            gamepad_state.insert(input, state);
        }
    }

    pub fn get(&self, input: Input) -> InputState {
        assert!(
            self.metadata.present_inputs.contains(&input),
            "Invalid input requested {:?}",
            input
        );

        let gamepad_state = self.state.lock().unwrap();
        gamepad_state.get(&input).cloned().unwrap_or_default()
    }
}
