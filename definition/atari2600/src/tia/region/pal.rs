use super::Region;
use crate::tia::color::TiaColor;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Pal;

impl Region for Pal {
    const TOTAL_SCANLINES: u16 = 312;

    fn frequency() -> Ratio<u32> {
        Ratio::new(17734475, 4)
    }

    fn color_to_srgb(color: TiaColor) -> palette::Srgb<u8> {
        todo!()
    }
}
