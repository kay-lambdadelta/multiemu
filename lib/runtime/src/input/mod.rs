use std::borrow::Cow;

use serde::{Deserialize, Serialize};
pub use virtual_gamepad::*;

mod virtual_gamepad;

/// ID for a gamepad
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct VirtualGamepadId(pub Cow<'static, str>);
