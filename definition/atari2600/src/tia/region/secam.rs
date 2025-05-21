use super::Region;
use crate::tia::color::TiaColor;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Secam;

impl Region for Secam {
    const REFRESH_RATE: Ratio<u32> = Ratio::new_raw(50, 1);
    const TOTAL_SCANLINES: u16 = 312;

    fn frequency() -> Ratio<u32> {
        todo!()
    }

    fn color_to_srgb(color: TiaColor) -> palette::Srgb<u8> {
        todo!()
    }
}
