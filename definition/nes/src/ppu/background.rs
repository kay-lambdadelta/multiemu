use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BackgroundState {
    pub pattern_table_index: u8,
    pub rendering_enabled: bool,

    // Shift registers
    pub pattern_low_shift: u16,
    pub pattern_high_shift: u16,
    pub attribute_shift: u32,
    /// Usually called x, 3 bits
    pub fine_x_scroll: u8,
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

#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum SpritePipelineState {
    FetchingNametableGarbage0,
    FetchingNametableGarbage1,
    FetchingPatternTableLow,
    FetchingPatternTableHigh { pattern_table_low: u8 },
}
