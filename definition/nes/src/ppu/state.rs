use std::sync::atomic::AtomicBool;

use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::ppu::{
    BackgroundPipelineState, ColorEmphasis,
    background::{BackgroundState, SpritePipelineState},
    oam::OamState,
};

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub sprite_size: Vector2<u16>,
    pub vblank_nmi_enabled: bool,
    pub greyscale: bool,
    pub entered_vblank: AtomicBool,
    pub show_background_leftmost_pixels: bool,
    /// NES documents tend to call this w
    pub vram_address_pointer_write_phase: bool,
    pub vram_address_pointer_increment_amount: u8,
    pub color_emphasis: ColorEmphasis,
    pub cycle_counter: Point2<u16>,
    pub awaiting_memory_access: bool,
    pub background_pipeline_state: BackgroundPipelineState,
    pub sprite_pipeline_state: SpritePipelineState,
    pub oam: OamState,
    pub background: BackgroundState,
    /// Actually 15 bits, usually called v
    pub vram_address_pointer: u16,
    /// Actually 15 bits, usually called t
    pub shadow_vram_address_pointer: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct VramAddressPointerContents {
    pub fine_y: u8,
    pub coarse: Vector2<u8>,
    pub nametable: Vector2<bool>,
}

const COARSE_X_MASK: u16 = 0b0000_0000_0001_1111;
const COARSE_Y_MASK: u16 = 0b0000_0011_1110_0000;
const NAMETABLE_H_MASK: u16 = 0b0000_0100_0000_0000;
const NAMETABLE_V_MASK: u16 = 0b0000_1000_0000_0000;
const FINE_Y_MASK: u16 = 0b0111_0000_0000_0000;

impl From<u16> for VramAddressPointerContents {
    #[inline]
    fn from(value: u16) -> Self {
        let coarse_x = (value & COARSE_X_MASK) as u8;
        let coarse_y = ((value & COARSE_Y_MASK) >> 5) as u8;
        let nametable_x = (value & NAMETABLE_H_MASK) != 0;
        let nametable_y = (value & NAMETABLE_V_MASK) != 0;
        let fine_y = ((value & FINE_Y_MASK) >> 12) as u8;

        Self {
            fine_y,
            coarse: Vector2::new(coarse_x, coarse_y),
            nametable: Vector2::new(nametable_x, nametable_y),
        }
    }
}

impl From<VramAddressPointerContents> for u16 {
    #[inline]
    fn from(value: VramAddressPointerContents) -> Self {
        let mut result = 0;

        result |= Self::from(value.coarse.x) & COARSE_X_MASK;
        result |= (Self::from(value.coarse.y) << 5) & COARSE_Y_MASK;
        if value.nametable.x {
            result |= NAMETABLE_H_MASK;
        }
        if value.nametable.y {
            result |= NAMETABLE_V_MASK;
        }
        result |= (Self::from(value.fine_y) << 12) & FINE_Y_MASK;

        result
    }
}
