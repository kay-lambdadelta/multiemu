use std::fmt::Debug;

use num::rational::Ratio;
use palette::Srgb;

pub mod ntsc;
pub mod pal;
pub mod secam;

pub trait Region: Send + Sync + Debug + 'static {
    const REFRESH_RATE: Ratio<u32>;
    const TOTAL_SCANLINES: u16;

    fn frequency() -> Ratio<u32>;

    fn color_to_srgb(hue: u8, luminosity: u8) -> Srgb<u8>;
}
