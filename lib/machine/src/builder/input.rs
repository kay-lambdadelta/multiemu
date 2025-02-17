use super::ComponentBuilder;
use crate::component::Component;
use multiemu_input::virtual_gamepad::{VirtualGamepadId, VirtualGamepadMetadata};

pub struct InputMetadata {}

impl<C: Component> ComponentBuilder<'_, C> {
    pub fn insert_gamepads(
        self,
        gamepad_metadata: impl IntoIterator<Item = (VirtualGamepadId, VirtualGamepadMetadata)>,
        gamepads: impl IntoIterator<Item = VirtualGamepadId>,
    ) -> Self {
        self
    }
}
