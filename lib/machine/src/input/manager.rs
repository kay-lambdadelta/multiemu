use super::virtual_gamepad::VirtualGamepad;
use std::{collections::HashMap, sync::Arc};

pub struct InputManager {
    gamepads: HashMap<u8, Arc<VirtualGamepad>>,
}
