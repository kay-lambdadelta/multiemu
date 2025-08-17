use super::Region;
use crate::ppu::color::PpuColor;
use nalgebra::Vector2;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Dendy;

impl Region for Dendy {
    fn master_clock() -> Ratio<u32> {
        todo!()
    }

    fn visible_scanline_dimensions() -> Vector2<u16> {
        todo!()
    }

    fn color_to_srgb(color: PpuColor) -> palette::Srgb<u8> {
        todo!()
    }
}
