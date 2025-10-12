use super::Region;
use crate::ppu::color::PpuColor;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Pal;

impl Region for Pal {
    const VBLANK_LENGTH: u16 = 0;
    const VISIBLE_SCANLINES: u16 = 0;

    fn master_clock() -> Ratio<u32> {
        Ratio::new(17734475, 4)
    }

    fn color_to_srgb(color: PpuColor) -> palette::Srgb<u8> {
        todo!()
    }
}
