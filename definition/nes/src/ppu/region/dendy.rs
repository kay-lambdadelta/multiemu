use super::Region;
use crate::ppu::color::PpuColor;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Dendy;

impl Region for Dendy {
    const REFRESH_RATE: Ratio<u32> = Ratio::new_raw(50, 1);

    fn frequency() -> Ratio<u32> {
        todo!()
    }

    fn color_to_srgb(color: PpuColor) -> palette::Srgb<u8> {
        todo!()
    }
}
