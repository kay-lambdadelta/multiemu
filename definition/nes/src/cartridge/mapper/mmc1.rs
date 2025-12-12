use multiemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    platform::Platform,
};

use crate::cartridge::NesCartridgeConfig;

#[derive(Debug)]
pub struct Mmc1;

impl Component for Mmc1 {}

#[derive(Debug)]
pub struct Mmc1Config<'a> {
    pub config: &'a NesCartridgeConfig,
}

impl<P: Platform> ComponentConfig<P> for Mmc1Config<'_> {
    type Component = Mmc1;

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        todo!()
    }
}
