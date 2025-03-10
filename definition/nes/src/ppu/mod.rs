use super::CPU_ADDRESS_SPACE;
use multiemu_machine::builder::ComponentBuilder;
use multiemu_machine::component::{Component, FromConfig, RuntimeEssentials};
use multiemu_machine::memory::AddressSpaceId;
use std::ops::Range;
use std::sync::Arc;

mod software;
#[cfg(all(feature = "vulkan", platform_desktop))]
mod vulkan;

const ASSIGNED_AREAS: [(AddressSpaceId, Range<usize>); 2] = [
    (CPU_ADDRESS_SPACE, 0x2000..0x2008),
    (CPU_ADDRESS_SPACE, 0x4014..0x4015),
];

// We store ppu state registers in normal struct sizes for easier gpu access

const PPUCTRL_ADDRESS: usize = 0x2000;
const PPUMASK_ADDRESS: usize = 0x2001;
const PPUSTATUS_ADDRESS: usize = 0x2002;
const OAMADDR_ADDRESS: usize = 0x2003;

pub struct OamData {}

impl OamData {
    const ADDRESS: usize = 0x2004;
}

const PPUSCROLL_ADDRESS: usize = 0x2005;
const PPUADDR_ADDRESS: usize = 0x2006;
const PPUDATA_ADDRESS: usize = 0x2007;
const OAMDMA_ADDRESS: usize = 0x4014;

struct State {
    oamdata: u8,
}

pub struct NesPpu {
    essentials: Arc<RuntimeEssentials>,
}

impl Component for NesPpu {}

impl FromConfig for NesPpu {
    type Config = ();

    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        _config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        component_builder.build(Self { essentials });
    }
}
