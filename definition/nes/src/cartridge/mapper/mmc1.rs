use crate::{INes, cartridge::ines::INesVersion};
use multiemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::AddressSpaceId,
    platform::Platform,
    program::RomId,
};

#[derive(Debug)]
pub struct Mmc1;

impl Component for Mmc1 {}

#[derive(Debug)]
pub struct Mmc1Config<'a> {
    pub ines: &'a INes,
    pub rom_id: RomId,
    pub cpu_address_space: AddressSpaceId,
    pub ppu_address_space: AddressSpaceId,
}

impl<'a, P: Platform> ComponentConfig<P> for Mmc1Config<'a> {
    type Component = Mmc1;

    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let submapper = match self.ines.version {
            INesVersion::V1 => None,
            INesVersion::V2 { submapper, .. } => Some(submapper),
        };

        todo!()
    }
}
