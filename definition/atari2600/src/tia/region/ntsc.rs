use std::sync::LazyLock;

use fluxemu_runtime::scheduler::Frequency;
use nalgebra::SMatrix;
use palette::{FromColor, Hsl, Srgb};

use super::Region;
use crate::tia::color::TiaColor;

// FIXME: Truly the worst math in this repository

static BASE_COLOR_PALETTE: LazyLock<[Hsl<palette::encoding::Srgb, f32>; 16]> =
    LazyLock::new(|| {
        [
            Hsl::new(0.0, 0.0, 0.0),
            Hsl::new(60.0, 1.0, 0.05),
            Hsl::new(21.0, 1.0, 0.11),
            Hsl::new(11.0, 1.0, 0.14),
            Hsl::new(0.0, 1.0, 0.13),
            Hsl::new(314.0, 1.0, 0.15),
            Hsl::new(276.0, 1.0, 0.25),
            Hsl::new(249.0, 1.0, 0.28),
            Hsl::new(240.0, 1.0, 0.22),
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
        for l in 0..8 {
            let lightness = (0.7 / 7.0) * l as f32;
            let hsl = Hsl::new(
                color.hue,
                color.saturation,
                (color.lightness + lightness).clamp(0.0, 1.0),
            );

            palette[(i, l)] = Srgb::from_color(hsl).into_format();
        }
    }

    palette
});

#[derive(Debug)]
pub struct Ntsc;

impl Region for Ntsc {
    const TOTAL_SCANLINES: u16 = 262;

    fn frequency() -> Frequency {
        Frequency::from_num(3579545)
    }

    fn color_to_srgb(color: TiaColor) -> Srgb<u8> {
        COLOR_PALETTE[(color.hue as usize, color.luminance as usize)]
    }
}
