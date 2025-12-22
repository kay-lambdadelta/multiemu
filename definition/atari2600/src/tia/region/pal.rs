use fluxemu_runtime::scheduler::Frequency;

use super::Region;
use crate::tia::color::TiaColor;

#[derive(Debug)]
pub struct Pal;

impl Region for Pal {
    const TOTAL_SCANLINES: u16 = 312;

    fn frequency() -> Frequency {
        Frequency::from_num(17734475) / 4
    }

    fn color_to_srgb(color: TiaColor) -> palette::Srgb<u8> {
        todo!()
    }
}
