use multiemu_input::{Input, InputState, VirtualGamepadName};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct VirtualGamepadMetadata {
    pub present_inputs: HashSet<Input>,
    pub default_bindings: HashMap<Input, Input>,
}

#[derive(Debug)]
pub struct VirtualGamepad {
    name: VirtualGamepadName,
    metadata: Arc<VirtualGamepadMetadata>,
    state: scc::HashMap<Input, InputState>,
}

impl VirtualGamepad {
    pub fn new(
        name: VirtualGamepadName,
        medadata: impl Into<Arc<VirtualGamepadMetadata>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name,
            metadata: medadata.into(),
            state: scc::HashMap::new(),
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
            let _ = self.state.upsert(input, state);
        }
    }

    pub fn get(&self, input: Input) -> InputState {
        assert!(
            self.metadata.present_inputs.contains(&input),
            "Invalid input requested {:?}",
            input
        );

        self.state
            .get(&input)
            .map(|state| *state.deref())
            .unwrap_or_default()
    }
}
