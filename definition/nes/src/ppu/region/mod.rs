use std::fmt::Debug;

use multiemu_runtime::scheduler::Frequency;
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

    fn master_clock() -> Frequency;
    fn color_to_srgb(color: PpuColor) -> Srgb<u8>;
}
