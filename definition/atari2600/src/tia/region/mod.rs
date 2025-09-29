use super::color::TiaColor;
use num::rational::Ratio;
use palette::Srgb;
use std::fmt::Debug;

pub mod ntsc;
pub mod pal;
pub mod secam;

pub trait Region: Send + Sync + Debug + 'static {
    const TOTAL_SCANLINES: u16;

    fn frequency() -> Ratio<u32>;

    fn color_to_srgb(color: TiaColor) -> Srgb<u8>;
}
