use num::rational::Ratio;

use super::Region;

pub struct Pal;
impl Region for Pal {
    const REFRESH_RATE: Ratio<u32> = Ratio::new_raw(50, 1);
    const TOTAL_SCANLINES: u16 = 312;

    fn frequency() -> Ratio<u32> {
        Ratio::new(17734475, 4)
    }

    fn color_to_srgb(hue: u8, luminosity: u8) -> palette::Srgb<u8> {
        todo!()
    }
}
