use crate::ppu::color::PpuColor;
use num::rational::Ratio;
use palette::Srgb;
use std::fmt::Debug;

pub mod dendy;
pub mod ntsc;
pub mod pal;

pub trait Region: Send + Sync + Debug + 'static {
    const REFRESH_RATE: Ratio<u32>;

    fn frequency() -> Ratio<u32>;
    fn color_to_srgb(color: PpuColor) -> Srgb<u8>;
}
