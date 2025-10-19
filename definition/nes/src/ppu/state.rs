use crate::ppu::oam::OamSprite;
use crate::ppu::{ColorEmphasis, PipelineState};
use arrayvec::ArrayVec;
use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serde_with::Bytes;
use serde_with::serde_as;
use std::sync::atomic::AtomicBool;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub nametable_base: u16,
    pub sprite_8x8_pattern_table_address: u16,
    pub background_pattern_table_base: u16,
    pub sprite_size: Vector2<u16>,
    pub vblank_nmi_enabled: bool,
    pub reset_cpu_nmi: bool,
    pub greyscale: bool,
    pub entered_vblank: AtomicBool,
    pub show_background_leftmost_pixels: bool,
    pub show_sprites_leftmost_pixels: bool,
    pub background_rendering_enabled: bool,
    pub sprite_rendering_enabled: bool,
    pub ppu_addr: u16,
    pub oam_addr: u8,
    // NES documents tend to call this w
    pub ppu_addr_ppu_scroll_write_phase: bool,
    pub ppu_addr_increment_amount: u8,
    pub color_emphasis: ColorEmphasis,
    pub cycle_counter: Point2<u16>,
    pub fine_scroll: Vector2<u8>,
    pub coarse_scroll: Vector2<u8>,
    pub awaiting_memory_access: bool,
    pub pipeline_state: PipelineState,
    #[serde_as(as = "Bytes")]
    pub oam_data: [u8; 256],
    pub queued_sprites: ArrayVec<OamSprite, 8>,
    // Shift registers
    pub pattern_low_shift: u16,
    pub pattern_high_shift: u16,
    pub attribute_shift: u32,
}
