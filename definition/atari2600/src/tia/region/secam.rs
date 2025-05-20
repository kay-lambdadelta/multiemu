use super::Region;
use num::rational::Ratio;

#[derive(Debug)]
pub struct Secam;

impl Region for Secam {
    const REFRESH_RATE: Ratio<u32> = Ratio::new_raw(50, 1);
    const TOTAL_SCANLINES: u16 = 312;

    fn frequency() -> Ratio<u32> {
        todo!()
    }

    fn color_to_srgb(hue: u8, luminosity: u8) -> palette::Srgb<u8> {
        todo!()
    }
}
