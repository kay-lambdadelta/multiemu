use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::Address,
};

mod software;
#[cfg(all(feature = "vulkan", platform_desktop))]
mod vulkan;

/*
const ASSIGNED_AREAS: [(AddressSpaceId, Range<usize>); 2] = [
    (CPU_ADDRESS_SPACE, 0x2000..0x2008),
    (CPU_ADDRESS_SPACE, 0x4014..0x4015),
];
*/

// We store ppu state registers in normal struct sizes for easier gpu access

const PPUCTRL_ADDRESS: Address = 0x2000;
const PPUMASK_ADDRESS: Address = 0x2001;
const PPUSTATUS_ADDRESS: Address = 0x2002;
const OAMADDR_ADDRESS: Address = 0x2003;

pub struct OamData {}

impl OamData {
    const ADDRESS: Address = 0x2004;
}

const PPUSCROLL_ADDRESS: Address = 0x2005;
const PPUADDR_ADDRESS: Address = 0x2006;
const PPUDATA_ADDRESS: Address = 0x2007;
const OAMDMA_ADDRESS: Address = 0x4014;

struct State {
    oamdata: u8,
}

#[derive(Default, Debug)]
pub struct NesPpuConfig;

#[derive(Debug)]
pub struct NesPpu;
impl Component for NesPpu {}

impl<R: RenderApi> ComponentConfig<R> for NesPpuConfig {
    type Component = NesPpu;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        component_builder.build(NesPpu);
    }
}
