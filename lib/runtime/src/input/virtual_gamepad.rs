use std::{borrow::Cow, collections::HashMap, sync::Arc};

use rustc_hash::FxBuildHasher;

use crate::input::{Input, InputState};

#[derive(Debug, Clone)]
/// Information a component gave about a emulated gamepad
pub struct VirtualGamepadMetadata {
    pub present_inputs: Vec<Input>,
    pub default_real2virtual_mappings: HashMap<Input, Input>,
}

#[derive(Debug)]
/// A emulated gamepad
pub struct VirtualGamepad {
    metadata: Cow<'static, VirtualGamepadMetadata>,
    state: scc::HashMap<Input, InputState, FxBuildHasher>,
}

impl VirtualGamepad {
    pub fn new(metadata: impl Into<Cow<'static, VirtualGamepadMetadata>>) -> Arc<Self> {
        Arc::new(Self {
            metadata: metadata.into(),
            state: Default::default(),
        })
    }

    pub fn metadata(&self) -> &VirtualGamepadMetadata {
        &self.metadata
    }

    pub fn set(&self, input: Input, state: InputState) {
        if self.metadata.present_inputs.contains(&input) {
            self.state.upsert_sync(input, state);
        }
    }

    pub fn get(&self, input: Input) -> InputState {
        self.state
            .get_sync(&input)
            .as_deref()
            .copied()
            .unwrap_or_default()
    }
}
