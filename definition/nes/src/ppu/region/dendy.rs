use super::Region;
use crate::ppu::color::PpuColor;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Dendy;

impl Region for Dendy {
    const VBLANK_LENGTH: u16 = 0;
    const VISIBLE_SCANLINES: u16 = 0;

    fn master_clock() -> Ratio<u32> {
        todo!()
    }

    fn color_to_srgb(color: PpuColor) -> palette::Srgb<u8> {
        todo!()
    }
}
