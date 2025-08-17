use crate::ppu::color::PpuColor;
use nalgebra::Vector2;
use num::rational::Ratio;
use palette::Srgb;
use std::fmt::Debug;

pub mod dendy;
pub mod ntsc;
pub mod pal;

pub trait Region: Send + Sync + Debug + 'static {
    fn master_clock() -> Ratio<u32>;
    fn visible_scanline_dimensions() -> Vector2<u16>;
    fn color_to_srgb(color: PpuColor) -> Srgb<u8>;
}
