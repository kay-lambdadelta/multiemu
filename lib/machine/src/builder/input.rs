use super::ComponentBuilder;
use crate::{component::Component, input::virtual_gamepad::VirtualGamepad};
use std::sync::Arc;

#[derive(Default)]
pub struct InputMetadata {
    pub gamepads: Vec<Arc<VirtualGamepad>>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    /// Insert a virtual gamepad for the runtime to be aware of
    pub fn insert_gamepads(
        mut self,
        gamepads: impl IntoIterator<Item = Arc<VirtualGamepad>>,
    ) -> Self {
        self.component_metadata
            .input
            .get_or_insert_default()
            .gamepads
            .extend(gamepads);

        self
    }
}
