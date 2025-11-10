use crate::ppu::background::{BackgroundState, SpritePipelineState};
use crate::ppu::oam::OamState;
use crate::ppu::{BackgroundPipelineState, ColorEmphasis};
use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::sync::atomic::AtomicBool;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub nametable_base: u16,
    pub sprite_size: Vector2<u16>,
    pub vblank_nmi_enabled: bool,
    pub reset_cpu_nmi: bool,
    pub greyscale: bool,
    pub entered_vblank: AtomicBool,
    pub show_background_leftmost_pixels: bool,
    pub ppu_addr: u16,
    // NES documents tend to call this w
    pub ppu_addr_ppu_scroll_write_phase: bool,
    pub ppu_addr_increment_amount: u8,
    pub color_emphasis: ColorEmphasis,
    pub cycle_counter: Point2<u16>,
    pub awaiting_memory_access: bool,
    pub background_pipeline_state: BackgroundPipelineState,
    pub sprite_pipeline_state: SpritePipelineState,
    pub oam: OamState,
    pub background: BackgroundState,
}
