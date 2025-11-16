use std::fmt::Debug;

use num::rational::Ratio;
use palette::Srgb;

use crate::ppu::{DUMMY_SCANLINE_COUNT, color::PpuColor};

pub mod dendy;
pub mod ntsc;
pub mod pal;

pub trait Region: Send + Sync + Debug + 'static {
    const VISIBLE_SCANLINES: u16;
    const VBLANK_LENGTH: u16;
    const TOTAL_SCANLINES: u16 =
        Self::VISIBLE_SCANLINES + Self::VBLANK_LENGTH + DUMMY_SCANLINE_COUNT;

    fn master_clock() -> Ratio<u32>;
    fn color_to_srgb(color: PpuColor) -> Srgb<u8>;
}
