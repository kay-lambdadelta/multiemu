use super::Region;
use crate::ppu::color::PpuColor;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Pal;

impl Region for Pal {
    fn master_clock() -> Ratio<u32> {
        Ratio::new(17734475, 4)
    }

    fn visible_scanline_dimensions() -> nalgebra::Vector2<u16> {
        todo!()
    }

    fn color_to_srgb(color: PpuColor) -> palette::Srgb<u8> {
        todo!()
    }
}
