use std::collections::HashMap;

use multiemu_runtime::{
    component::ResourcePath,
    input::{Input, RealGamepadId},
    program::MachineId,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Real2VirtualMappings(pub HashMap<RealGamepadId, HashMap<Input, Input>>);

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GamepadConfigs {
    pub gamepads: HashMap<(MachineId, ResourcePath), Real2VirtualMappings>,
}
