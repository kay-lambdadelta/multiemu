use std::ops::RangeInclusive;

use fluxemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId},
    platform::Platform,
};

#[derive(Debug, Clone)]
pub struct NullMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    // Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<Address>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceId,
}

impl<P: Platform> ComponentConfig<P> for NullMemoryConfig {
    type Component = NullMemory;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        if self.assigned_range.is_empty() {
            return Err("Memory assigned must be non-empty".into());
        }

        match (self.readable, self.writable) {
            (true, true) => component_builder
                .memory_map_component(self.assigned_address_space, self.assigned_range.clone()),
            (true, false) => component_builder.memory_map_component_read(
                self.assigned_address_space,
                self.assigned_range.clone(),
            ),
            (false, true) => component_builder.memory_map_component_write(
                self.assigned_address_space,
                self.assigned_range.clone(),
            ),
            (false, false) => component_builder,
        };

        Ok(NullMemory)
    }
}

#[derive(Debug)]
/// Always denies accesses, if you need this for some reason it exists
pub struct NullMemory;

impl Component for NullMemory {}
