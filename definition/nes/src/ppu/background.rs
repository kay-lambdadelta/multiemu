use nalgebra::Vector2;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BackgroundState {
    pub fine_scroll: Vector2<u8>,
    pub coarse_scroll: Vector2<u8>,
    pub pattern_table_base: u16,
    pub rendering_enabled: bool,

    // Shift registers
    pub pattern_low_shift: u16,
    pub pattern_high_shift: u16,
    pub attribute_shift: u32,
}

#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum BackgroundPipelineState {
    FetchingNametable,
    FetchingAttribute {
        nametable: u8,
    },
    FetchingPatternTableLow {
        nametable: u8,
        attribute: u8,
    },
    FetchingPatternTableHigh {
        nametable: u8,
        attribute: u8,
        pattern_table_low: u8,
    },
}
