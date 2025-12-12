use std::collections::HashMap;

use multiemu_runtime::{
    input::{Input, RealGamepadId},
    path::MultiemuPath,
    program::MachineId,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Real2VirtualMappings(pub HashMap<RealGamepadId, HashMap<Input, Input>>);

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GamepadConfigs {
    pub gamepads: HashMap<(MachineId, MultiemuPath), Real2VirtualMappings>,
}
