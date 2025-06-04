use crate::input::virtual_gamepad::VirtualGamepad;
use std::{sync::Arc, vec::Vec};

#[derive(Default)]
pub struct InputMetadata {
    pub gamepads: Vec<Arc<VirtualGamepad>>,
}
