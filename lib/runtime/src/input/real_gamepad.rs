use std::{borrow::Cow, fmt::Display, sync::Arc};

use crossbeam::atomic::AtomicCell;
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};
use uuid::{NonNilUuid, Uuid};

use crate::input::{Input, InputState};

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
/// The ID of a real gamepad
pub struct RealGamepadId(pub Uuid);

impl RealGamepadId {
    /// The ID of the platforms default input device
    ///
    /// For desktop operating systems, this is the keyboard
    ///
    /// For handheld consoles with abnormal operating systems this is the built
    /// in gamepad
    pub const PLATFORM_RESERVED: RealGamepadId = RealGamepadId(Uuid::from_u128(0));

    /// Creates a new gamepad ID
    pub const fn new(id: NonNilUuid) -> Self {
        Self(id.get())
    }
}

impl Display for RealGamepadId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
/// A emulated gamepad
pub struct RealGamepad {
    metadata: RealGamepadMetadata,
    state: scc::HashMap<Input, InputState, FxBuildHasher>,
    battery_level: AtomicCell<Option<f32>>,
}

impl RealGamepad {
    pub fn new(metadata: RealGamepadMetadata) -> Arc<Self> {
        Arc::new(Self {
            metadata,
            state: Default::default(),
            battery_level: AtomicCell::default(),
        })
    }

    pub fn metadata(&self) -> &RealGamepadMetadata {
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

    pub fn get_battery_level(&self) -> Option<f32> {
        self.battery_level.load()
    }

    pub fn set_battery_level(&self, battery_level: f32) {
        self.battery_level
            .store(Some(battery_level.clamp(0.0, 1.0)));
    }
}

#[derive(Debug, Clone)]
/// Information a component gave about a emulated gamepad
pub struct RealGamepadMetadata {
    pub name: Cow<'static, str>,
    pub present_inputs: Vec<Input>,
}
