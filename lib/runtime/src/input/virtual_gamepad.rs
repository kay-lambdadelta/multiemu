use multiemu_input::{Input, InputState, VirtualGamepadName};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
/// Information a component gave about a emulated gamepad
pub struct VirtualGamepadMetadata {
    pub present_inputs: HashSet<Input>,
    pub default_bindings: HashMap<Input, Input>,
}

#[derive(Debug)]
/// A emulated gamepad
pub struct VirtualGamepad {
    name: VirtualGamepadName,
    metadata: Arc<VirtualGamepadMetadata>,
    state: RwLock<HashMap<Input, InputState>>,
}

impl VirtualGamepad {
    pub fn new(
        name: VirtualGamepadName,
        medadata: impl Into<Arc<VirtualGamepadMetadata>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name,
            metadata: medadata.into(),
            state: RwLock::default(),
        })
    }

    pub fn name(&self) -> VirtualGamepadName {
        self.name.clone()
    }

    pub fn metadata(&self) -> &VirtualGamepadMetadata {
        &self.metadata
    }

    pub fn set(&self, input: Input, state: InputState) {
        if self.metadata.present_inputs.contains(&input) {
            self.state.write().unwrap().insert(input, state);
        }
    }

    pub fn get(&self, input: Input) -> InputState {
        assert!(
            self.metadata.present_inputs.contains(&input),
            "Invalid input requested {:?}",
            input
        );

        self.state
            .read()
            .unwrap()
            .get(&input)
            .copied()
            .unwrap_or_default()
    }
}
