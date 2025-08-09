use super::Region;
use crate::ppu::color::PpuColor;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Pal;

impl Region for Pal {
    const REFRESH_RATE: Ratio<u32> = Ratio::new_raw(50, 1);

    fn frequency() -> Ratio<u32> {
        Ratio::new(17734475, 4)
    }

    fn color_to_srgb(color: PpuColor) -> palette::Srgb<u8> {
        todo!()
    }
}
