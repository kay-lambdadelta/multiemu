use super::Region;
use crate::ppu::color::PpuColor;
use nalgebra::SMatrix;
use num::rational::Ratio;
use palette::{FromColor, Hsl, Srgb};
use std::sync::LazyLock;

static COLOR_PALETTE: LazyLock<SMatrix<Srgb<u8>, 16, 4>> = LazyLock::new(|| {
    let mut palette = SMatrix::default();

    for hue in 0..16 {
        let hue_deg = (hue as f32) * 30.0;

        for lum in 0..4 {
            let lightness = ((lum + 1) as f32 * 0.20) + 0.05;

            let saturation = 0.9;

            let hsl = Hsl::new(hue_deg, saturation, lightness);
            let rgb: Srgb<u8> = Srgb::from_color(hsl).into_format();

            palette[(hue, lum)] = rgb;
        }
    }

    palette
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
