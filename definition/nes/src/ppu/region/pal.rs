use super::Region;
use crate::ppu::color::PpuColor;
use multiemu_runtime::scheduler::Frequency;

#[derive(Debug)]
pub struct Pal;

impl Region for Pal {
    const VBLANK_LENGTH: u16 = 0;
    const VISIBLE_SCANLINES: u16 = 0;

    fn master_clock() -> Frequency {
        Frequency::from_num(17734475) / 4
    }

    fn color_to_srgb(color: PpuColor) -> palette::Srgb<u8> {
        todo!()
    }
}
