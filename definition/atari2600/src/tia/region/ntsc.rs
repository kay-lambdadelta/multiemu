use super::Region;
use nalgebra::SMatrix;
use num::rational::Ratio;
use palette::{FromColor, Hsl, Srgb};
use std::sync::LazyLock;

static BASE_COLOR_PALETTE: LazyLock<[Hsl<Srgb, f32>; 16]> = LazyLock::new(|| {
    [
        Hsl::new(0.0, 0.0, 0.0),
        Hsl::new(60.0, 1.0, 0.13),
        Hsl::new(21.0, 1.0, 0.22),
        Hsl::new(11.0, 1.0, 0.26),
        Hsl::new(0.0, 1.0, 0.27),
        Hsl::new(314.0, 1.0, 0.24),
        Hsl::new(276.0, 1.0, 0.24),
        Hsl::new(249.0, 1.0, 0.26),
        Hsl::new(240.0, 1.0, 0.27),
        Hsl::new(228.0, 1.0, 0.24),
        Hsl::new(211.0, 1.0, 0.18),
        Hsl::new(161.0, 1.0, 0.13),
        Hsl::new(120.0, 1.0, 0.12),
        Hsl::new(99.0, 1.0, 0.11),
        Hsl::new(65.0, 1.0, 0.09),
        Hsl::new(35.0, 1.0, 0.13),
    ]
});

static COLOR_PALETTE: LazyLock<SMatrix<Srgb<u8>, 16, 8>> = LazyLock::new(|| {
    let mut palette = SMatrix::default();

    for (i, color) in BASE_COLOR_PALETTE.iter().enumerate() {
        for sat in 0..8 {
            let saturation = (sat as f32) / 7.0;
            let hsl = Hsl::new(color.hue, saturation, color.lightness);

            palette[(i, sat)] = Srgb::from_color(hsl).into_format();
        }
    }

    palette
});

pub struct Ntsc;
impl Region for Ntsc {
    const REFRESH_RATE: Ratio<u32> = Ratio::new_raw(60, 1);
    const TOTAL_SCANLINES: u16 = 262;

    fn frequency() -> Ratio<u32> {
        Ratio::new(3579545, 1)
    }

    fn color_to_srgb(hue: u8, luminosity: u8) -> Srgb<u8> {
        COLOR_PALETTE[(hue as usize, luminosity as usize)]
    }
}
