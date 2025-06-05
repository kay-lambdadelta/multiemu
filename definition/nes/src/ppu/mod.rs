use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    memory::Address,
};

mod software;
#[cfg(feature = "vulkan")]
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

impl<B: ComponentBuilder<Component = NesPpu>> ComponentConfig<B> for NesPpuConfig {
    type Component = NesPpu;

    fn build_component(self, component_builder: B) -> B::BuildOutput {
        component_builder.build(NesPpu)
    }
}
