use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, ComponentRef},
    memory::{Address, AddressSpaceHandle},
    platform::Platform,
};
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct NullMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    // Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<Address>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceHandle,
}

impl<P: Platform> ComponentConfig<P> for NullMemoryConfig {
    type Component = NullMemory;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) {
        let component_builder = match (self.readable, self.writable) {
            (true, true) => component_builder
                .map_memory([(self.assigned_address_space, self.assigned_range.clone())]),
            (true, false) => component_builder
                .map_memory_read([(self.assigned_address_space, self.assigned_range.clone())]),
            (false, true) => component_builder
                .map_memory_write([(self.assigned_address_space, self.assigned_range.clone())]),
            (false, false) => component_builder,
        };

        component_builder.build_global(NullMemory)
    }
}

#[derive(Debug)]
/// Always denies accesses, if you need this for some reason it exists
pub struct NullMemory;

impl Component for NullMemory {}
