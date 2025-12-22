use std::fmt::Debug;

use fluxemu_runtime::scheduler::Frequency;
use palette::Srgb;

use super::color::TiaColor;

pub mod ntsc;
pub mod pal;
pub mod secam;

pub trait Region: Send + Sync + Debug + 'static {
    const TOTAL_SCANLINES: u16;

    fn frequency() -> Frequency;

    fn color_to_srgb(color: TiaColor) -> Srgb<u8>;
}
