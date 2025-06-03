use super::ComponentBuilder;
use crate::{
    component::Component, display::backend::RenderApi, input::virtual_gamepad::VirtualGamepad,
};
use std::{sync::Arc, vec::Vec};

#[derive(Default)]
pub struct InputMetadata {
    pub gamepads: Vec<Arc<VirtualGamepad>>,
}

impl<R: RenderApi, C: Component> ComponentBuilder<'_, R, C> {
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
