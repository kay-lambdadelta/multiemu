use std::sync::atomic::{AtomicBool, AtomicU8};

use multiemu_runtime::{
    memory::{AddressSpace, AddressSpaceCache},
    scheduler::Period,
};
use nalgebra::{Point2, Vector2};
use palette::Srgb;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::ppu::{
    ATTRIBUTE_BASE_ADDRESS, BACKGROUND_PALETTE_BASE_ADDRESS, BackgroundPipelineState,
    ColorEmphasis, NAMETABLE_BASE_ADDRESS, SPRITE_PALETTE_BASE_ADDRESS,
    background::{BackgroundState, SpritePipelineState},
    color::PpuColor,
    oam::{CurrentlyRenderingSprite, OamSprite, OamState},
    region::Region,
};

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub sprite_size: Vector2<u16>,
    pub vblank_nmi_enabled: bool,
    pub greyscale: bool,
    pub entered_vblank: AtomicBool,
    pub show_background_leftmost_pixels: bool,
    /// NES documents tend to call this w
    pub vram_address_pointer_write_phase: bool,
    pub vram_address_pointer_increment_amount: u8,
    pub vram_read_buffer: AtomicU8,
    pub color_emphasis: ColorEmphasis,
    pub cycle_counter: Point2<u16>,
    pub awaiting_memory_access: bool,
    pub background_pipeline_state: BackgroundPipelineState,
    pub sprite_pipeline_state: SpritePipelineState,
    pub oam: OamState,
    pub background: BackgroundState,
    /// Actually 15 bits, usually called v
    pub vram_address_pointer: u16,
    /// Actually 15 bits, usually called t
    pub shadow_vram_address_pointer: u16,
}

impl State {
    #[inline]
    pub(crate) fn drive_sprite_pipeline<R: Region>(
        &mut self,
        ppu_address_space: &AddressSpace,
        ppu_address_space_cache: &mut AddressSpaceCache,
        timestamp: Period,
    ) {
        if !self.awaiting_memory_access {
            let currently_relevant_sprite_index = (self.cycle_counter.x - 257) / 8;
            let currently_relevant_sprite = self
                .oam
                .secondary_data
                .get(usize::from(currently_relevant_sprite_index));

            match self.sprite_pipeline_state {
                SpritePipelineState::FetchingNametableGarbage0 => {
                    self.sprite_pipeline_state = SpritePipelineState::FetchingNametableGarbage1;
                }
                SpritePipelineState::FetchingNametableGarbage1 => {
                    self.sprite_pipeline_state = SpritePipelineState::FetchingPatternTableLow;
                }
                SpritePipelineState::FetchingPatternTableLow => {
                    if let Some(currently_relevant_sprite) = currently_relevant_sprite {
                        let mut row = (self.cycle_counter.y
                            - u16::from(currently_relevant_sprite.position.y))
                            % 8;
                        if currently_relevant_sprite.flip.y {
                            row = 7 - row;
                        }

                        let address = u16::from(self.oam.sprite_8x8_pattern_table_index) * 0x1000
                            + u16::from(currently_relevant_sprite.tile_index) * 16
                            + row;

                        let pattern_table_low = ppu_address_space
                            .read_le_value(
                                address as usize,
                                timestamp,
                                Some(ppu_address_space_cache),
                            )
                            .unwrap();

                        self.sprite_pipeline_state =
                            SpritePipelineState::FetchingPatternTableHigh { pattern_table_low };
                    } else {
                        self.sprite_pipeline_state =
                            SpritePipelineState::FetchingPatternTableHigh {
                                // This is a garbage value, it isn't used
                                pattern_table_low: 0x00,
                            };
                    }
                }
                SpritePipelineState::FetchingPatternTableHigh { pattern_table_low } => {
                    if let Some(currently_relevant_sprite) = currently_relevant_sprite {
                        let mut row = (self.cycle_counter.y
                            - u16::from(currently_relevant_sprite.position.y))
                            % 8;
                        if currently_relevant_sprite.flip.y {
                            row = 7 - row;
                        }

                        let address = u16::from(self.oam.sprite_8x8_pattern_table_index) * 0x1000
                            + u16::from(currently_relevant_sprite.tile_index) * 16
                            + row
                            + 8;

                        let pattern_table_high = ppu_address_space
                            .read_le_value(
                                address as usize,
                                timestamp,
                                Some(ppu_address_space_cache),
                            )
                            .unwrap();

                        self.oam
                            .currently_rendering_sprites
                            .push(CurrentlyRenderingSprite {
                                oam: *currently_relevant_sprite,
                                pattern_table_high,
                                pattern_table_low,
                            });
                    }

                    self.sprite_pipeline_state = SpritePipelineState::FetchingNametableGarbage0;
                }
            }
        }

        self.awaiting_memory_access = !self.awaiting_memory_access;
    }

    #[inline]
    pub(crate) fn drive_background_pipeline<R: Region>(
        &mut self,
        ppu_address_space: &AddressSpace,
        ppu_address_space_cache: &mut AddressSpaceCache,
        timestamp: Period,
    ) {
        // Steps wait a cycle inbetween for memory access realism
        if !self.awaiting_memory_access {
            let mut vram_address_pointer_contents =
                VramAddressPointerContents::from(self.vram_address_pointer);

            match self.background_pipeline_state {
                BackgroundPipelineState::FetchingNametable => {
                    let nametable_index = (vram_address_pointer_contents.nametable.y as usize) * 2
                        + (vram_address_pointer_contents.nametable.x as usize);

                    let nametable_base = NAMETABLE_BASE_ADDRESS + (nametable_index * 0x400);

                    let address = nametable_base
                        + (usize::from(vram_address_pointer_contents.coarse.y) * 32)
                        + usize::from(vram_address_pointer_contents.coarse.x);

                    let nametable = ppu_address_space
                        .read_le_value(address, timestamp, Some(ppu_address_space_cache))
                        .unwrap();

                    self.background_pipeline_state =
                        BackgroundPipelineState::FetchingAttribute { nametable };
                }
                BackgroundPipelineState::FetchingAttribute { nametable } => {
                    let coarse = vram_address_pointer_contents.coarse.cast::<usize>();
                    let nametable_index = (vram_address_pointer_contents.nametable.y as usize) * 2
                        + (vram_address_pointer_contents.nametable.x as usize);

                    // Attribute table is every 4x4 tiles
                    let address = ATTRIBUTE_BASE_ADDRESS
                        + (nametable_index * 0x400)
                        + (coarse.y / 4) * 8
                        + (coarse.x / 4);

                    let attribute: u8 = ppu_address_space
                        .read_le_value(address, timestamp, Some(ppu_address_space_cache))
                        .unwrap();

                    let attribute_quadrant = Point2::new(coarse.x % 4, coarse.y % 4) / 2;
                    let shift = (attribute_quadrant.y * 2 + attribute_quadrant.x) * 2;
                    let attribute = (attribute >> shift) & 0b11;

                    self.background_pipeline_state =
                        BackgroundPipelineState::FetchingPatternTableLow {
                            nametable,
                            attribute,
                        };
                }
                BackgroundPipelineState::FetchingPatternTableLow {
                    nametable,
                    attribute,
                } => {
                    let pattern_table_base =
                        u16::from(self.background.pattern_table_index) * 0x1000;

                    let address = pattern_table_base
                        + (u16::from(nametable) * 16)
                        + u16::from(vram_address_pointer_contents.fine_y);

                    let pattern_table_low = ppu_address_space
                        .read_le_value(address as usize, timestamp, Some(ppu_address_space_cache))
                        .unwrap();

                    self.background_pipeline_state =
                        BackgroundPipelineState::FetchingPatternTableHigh {
                            nametable,
                            attribute,
                            pattern_table_low,
                        };
                }
                BackgroundPipelineState::FetchingPatternTableHigh {
                    nametable,
                    attribute,
                    pattern_table_low,
                } => {
                    let pattern_table_base =
                        u16::from(self.background.pattern_table_index) * 0x1000;

                    let address = pattern_table_base
                        + (u16::from(nametable) * 16)
                        + u16::from(vram_address_pointer_contents.fine_y)
                        + 8;

                    let pattern_table_high: u8 = ppu_address_space
                        .read_le_value(address as usize, timestamp, Some(ppu_address_space_cache))
                        .unwrap();

                    self.background.pattern_low_shift =
                        (self.background.pattern_low_shift & 0xff00) | u16::from(pattern_table_low);
                    self.background.pattern_high_shift = (self.background.pattern_high_shift
                        & 0xff00)
                        | u16::from(pattern_table_high);
                    self.background.attribute_shift = (self.background.attribute_shift
                        & 0xffff_0000)
                        // Spread the bits
                        | (u32::from(attribute) * 0x5555);

                    if self.background.rendering_enabled {
                        if vram_address_pointer_contents.coarse.x == 31 {
                            vram_address_pointer_contents.coarse.x = 0;
                            vram_address_pointer_contents.nametable.x =
                                !vram_address_pointer_contents.nametable.x;
                        } else {
                            vram_address_pointer_contents.coarse.x += 1;
                        }

                        self.vram_address_pointer = vram_address_pointer_contents.into();
                    }

                    self.background_pipeline_state = BackgroundPipelineState::FetchingNametable;
                }
            }
        }

        self.awaiting_memory_access = !self.awaiting_memory_access;
    }

    // This function uses manual bit math because absolute speed is critical here

    #[inline]
    pub fn calculate_background_color<R: Region>(
        &self,
        ppu_address_space: &AddressSpace,
        ppu_address_space_cache: &mut AddressSpaceCache,
        timestamp: Period,
        attribute: u8,
        color: u8,
    ) -> Srgb<u8> {
        let color_bits = color & 0b11;

        // Combine into a 4-bit palette index
        let palette_index = color_bits | (attribute << 2);

        let color_value: u8 = ppu_address_space
            .read_le_value(
                BACKGROUND_PALETTE_BASE_ADDRESS + palette_index as usize,
                timestamp,
                Some(ppu_address_space_cache),
            )
            .unwrap();

        let color = PpuColor {
            hue: color_value & 0b1111,
            luminance: (color_value >> 4) & 0b11,
        };

        R::color_to_srgb(color)
    }

    #[inline]
    pub fn calculate_sprite_color<R: Region>(
        &self,
        ppu_address_space: &AddressSpace,
        ppu_address_space_cache: &mut AddressSpaceCache,
        timestamp: Period,
        sprite: OamSprite,
        color: u8,
    ) -> Srgb<u8> {
        let color_bits = color & 0b11;

        let color_value: u8 = ppu_address_space
            .read_le_value(
                SPRITE_PALETTE_BASE_ADDRESS
                    + (usize::from(sprite.palette_index) * 4)
                    + usize::from(color_bits),
                timestamp,
                Some(ppu_address_space_cache),
            )
            .unwrap();

        let color = PpuColor {
            hue: color_value & 0b1111,
            luminance: (color_value >> 4) & 0b11,
        };

        R::color_to_srgb(color)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VramAddressPointerContents {
    pub fine_y: u8,
    pub coarse: Vector2<u8>,
    pub nametable: Vector2<bool>,
}

const COARSE_X_MASK: u16 = 0b0000_0000_0001_1111;
const COARSE_Y_MASK: u16 = 0b0000_0011_1110_0000;
const NAMETABLE_H_MASK: u16 = 0b0000_0100_0000_0000;
const NAMETABLE_V_MASK: u16 = 0b0000_1000_0000_0000;
const FINE_Y_MASK: u16 = 0b0111_0000_0000_0000;

impl From<u16> for VramAddressPointerContents {
    #[inline]
    fn from(value: u16) -> Self {
        let coarse_x = (value & COARSE_X_MASK) as u8;
        let coarse_y = ((value & COARSE_Y_MASK) >> 5) as u8;
        let nametable_x = (value & NAMETABLE_H_MASK) != 0;
        let nametable_y = (value & NAMETABLE_V_MASK) != 0;
        let fine_y = ((value & FINE_Y_MASK) >> 12) as u8;

        Self {
            fine_y,
            coarse: Vector2::new(coarse_x, coarse_y),
            nametable: Vector2::new(nametable_x, nametable_y),
        }
    }
}

impl From<VramAddressPointerContents> for u16 {
    #[inline]
    fn from(value: VramAddressPointerContents) -> Self {
        let mut result = 0;

        result |= Self::from(value.coarse.x) & COARSE_X_MASK;
        result |= (Self::from(value.coarse.y) << 5) & COARSE_Y_MASK;
        if value.nametable.x {
            result |= NAMETABLE_H_MASK;
        }
        if value.nametable.y {
            result |= NAMETABLE_V_MASK;
        }
        result |= (Self::from(value.fine_y) << 12) & FINE_Y_MASK;

        result
    }
}
