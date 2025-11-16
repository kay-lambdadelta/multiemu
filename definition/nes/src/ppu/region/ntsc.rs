use std::sync::LazyLock;

use nalgebra::SMatrix;
use num::rational::Ratio;
use palette::Srgb;

use super::Region;
use crate::ppu::color::PpuColor;

#[rustfmt::skip]
pub static COLOR_PALETTE: LazyLock<SMatrix<Srgb<u8>, 16, 4>> = LazyLock::new(|| {
    SMatrix::from_row_slice(&[
        Srgb::new(84, 84, 84),
        Srgb::new(152, 150, 152),
        Srgb::new(236, 238, 236),
        Srgb::new(236, 238, 236),

        Srgb::new(0, 30, 116),
        Srgb::new(8, 76, 196),
        Srgb::new(76, 154, 236),
        Srgb::new(168, 204, 236),

        Srgb::new(8, 16, 144),
        Srgb::new(48, 50, 236),
        Srgb::new(120, 124, 236),
        Srgb::new(188, 188, 236),

        Srgb::new(48, 0, 136),
        Srgb::new(92, 30, 228),
        Srgb::new(176, 98, 236),
        Srgb::new(212, 178, 236),

        Srgb::new(68, 0, 100),
        Srgb::new(136, 20, 176),
        Srgb::new(228, 84, 236),
        Srgb::new(236, 174, 236),

        Srgb::new(92, 0, 48),
        Srgb::new(160, 20, 100),
        Srgb::new(236, 88, 180),
        Srgb::new(236, 174, 212),

        Srgb::new(84, 4, 0),
        Srgb::new(152, 34, 32),
        Srgb::new(236, 106, 100),
        Srgb::new(236, 180, 176),

        Srgb::new(60, 24, 0),
        Srgb::new(120, 60, 0),
        Srgb::new(212, 136, 32),
        Srgb::new(228, 196, 144),

        Srgb::new(32, 42, 0),
        Srgb::new(84, 90, 0),
        Srgb::new(160, 170, 0),
        Srgb::new(204, 210, 120),

        Srgb::new(8, 58, 0),
        Srgb::new(40, 114, 0),
        Srgb::new(116, 196, 0),
        Srgb::new(180, 222, 120),

        Srgb::new(0, 64, 0),
        Srgb::new(8, 124, 0),
        Srgb::new(76, 208, 32),
        Srgb::new(168, 226, 144),

        Srgb::new(0, 60, 0),
        Srgb::new(0, 118, 40),
        Srgb::new(56, 204, 108),
        Srgb::new(152, 226, 180),

        Srgb::new(0, 50, 60),
        Srgb::new(0, 102, 120),
        Srgb::new(56, 180, 204),
        Srgb::new(160, 214, 228),

        Srgb::new(0, 0, 0),
        Srgb::new(0, 0, 0),
        Srgb::new(60, 60, 60),
        Srgb::new(160, 162, 160),

        Srgb::new(0, 0, 0),
        Srgb::new(0, 0, 0),
        Srgb::new(0, 0, 0),
        Srgb::new(0, 0, 0),

        Srgb::new(0, 0, 0),
        Srgb::new(0, 0, 0),
        Srgb::new(0, 0, 0),
        Srgb::new(0, 0, 0),
    ])
});

#[derive(Debug)]
pub struct Ntsc;

impl Region for Ntsc {
    const VISIBLE_SCANLINES: u16 = 240;
    const VBLANK_LENGTH: u16 = 20;

    fn master_clock() -> Ratio<u32> {
        // 236.25 MHz / 11
        Ratio::new(236250000, 11)
    }

    #[inline]
    fn color_to_srgb(color: PpuColor) -> Srgb<u8> {
        COLOR_PALETTE[(color.hue as usize, color.luminance as usize)]
    }
}
