use std::{
    num::NonZero,
    ops::RangeInclusive,
    sync::{Arc, atomic::Ordering},
};

use multiemu_definition_mos6502::NmiFlag;
use multiemu_range::ContiguousRange;
use multiemu_runtime::{memory::AddressSpace, scheduler::Task};
use nalgebra::Point2;
use palette::{Srgb, named::BLACK};

use crate::ppu::{
    ATTRIBUTE_BASE_ADDRESS, BACKGROUND_PALETTE_BASE_ADDRESS, NAMETABLE_BASE_ADDRESS, Ppu,
    SPRITE_PALETTE_BASE_ADDRESS, State, TOTAL_SCANLINE_LENGTH,
    backend::{PpuDisplayBackend, SupportedGraphicsApiPpu},
    background::{BackgroundPipelineState, SpritePipelineState},
    color::PpuColor,
    oam::{CurrentlyRenderingSprite, OamSprite, SpriteEvaluationState},
    region::Region,
    state::VramAddressPointerContents,
};

pub struct Driver {
    pub processor_nmi: Arc<NmiFlag>,
    pub ppu_address_space: Arc<AddressSpace>,
}

impl<R: Region, G: SupportedGraphicsApiPpu> Task<Ppu<R, G>> for Driver {
    fn run(&mut self, component: &mut Ppu<R, G>, time_slice: NonZero<u32>) {
        let backend = component.backend.as_mut().unwrap();

        for _ in 0..time_slice.get() {
            if component.state.cycle_counter.y == 241 && component.state.cycle_counter.x == 1 {
                component
                    .state
                    .entered_vblank
                    .store(true, Ordering::Release);

                if component.state.vblank_nmi_enabled {
                    self.processor_nmi.store(false);
                }
            }

            if component.state.cycle_counter.y == 261 {
                if component.state.cycle_counter.x == 1 {
                    component
                        .state
                        .entered_vblank
                        .store(false, Ordering::Release);

                    self.processor_nmi.store(true);

                    backend.commit_staging_buffer();
                }

                if component.state.cycle_counter.x == 257
                    && component.state.background.rendering_enabled
                {
                    let t = VramAddressPointerContents::from(
                        component.state.shadow_vram_address_pointer,
                    );
                    let mut v =
                        VramAddressPointerContents::from(component.state.vram_address_pointer);

                    v.nametable.x = t.nametable.x;
                    v.coarse.x = t.coarse.x;

                    component.state.vram_address_pointer = u16::from(v);
                }

                if let 280..=304 = component.state.cycle_counter.x
                    && component.state.background.rendering_enabled
                {
                    let t = VramAddressPointerContents::from(
                        component.state.shadow_vram_address_pointer,
                    );
                    let mut v =
                        VramAddressPointerContents::from(component.state.vram_address_pointer);

                    v.nametable.y = t.nametable.y;
                    v.coarse.y = t.coarse.y;
                    v.fine_y = t.fine_y;

                    component.state.vram_address_pointer = u16::from(v);
                }

                if let 305..=320 = component.state.cycle_counter.x
                    && component.state.background.rendering_enabled
                {
                    component
                        .state
                        .drive_background_pipeline::<R>(&self.ppu_address_space);
                }
            }

            if (0..R::VISIBLE_SCANLINES).contains(&component.state.cycle_counter.y) {
                if component.state.cycle_counter.x == 1 {
                    // Technically the NES does it over 64 cycles
                    component.state.oam.secondary_data.clear();
                }

                if let 1..=256 = component.state.cycle_counter.x {
                    let scanline_position_x = component.state.cycle_counter.x - 1;

                    component
                        .state
                        .drive_background_pipeline::<R>(&self.ppu_address_space);

                    let mut sprite_pixel = None;

                    let potential_sprite = component
                        .state
                        .oam
                        .currently_rendering_sprites
                        .iter()
                        .rev()
                        .find_map(|sprite| {
                            let in_sprite_position = u16::from(sprite.oam.position.x)
                                .checked_sub(scanline_position_x)?;

                            if in_sprite_position < 8 {
                                let in_sprite_position = if !sprite.oam.flip.x {
                                    in_sprite_position
                                } else {
                                    7 - in_sprite_position
                                };

                                let low = (sprite.pattern_table_low >> in_sprite_position) & 1;
                                let high = (sprite.pattern_table_high >> in_sprite_position) & 1;
                                let color_index = (high << 1) | low;

                                if color_index != 0 {
                                    Some((sprite, color_index))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        });

                    if let Some((sprite, color_index)) = potential_sprite {
                        sprite_pixel = Some(component.state.calculate_sprite_color::<R>(
                            &self.ppu_address_space,
                            sprite.oam,
                            color_index,
                        ));
                    }

                    // Extract out and combine pattern bits
                    let high = (component.state.background.pattern_high_shift
                        >> (15 - component.state.background.fine_x_scroll))
                        & 1;

                    let low = (component.state.background.pattern_low_shift
                        >> (15 - component.state.background.fine_x_scroll))
                        & 1;

                    let color_index = (high << 1) | low;

                    // Extract out attribute bits
                    let attribute = (component.state.background.attribute_shift >> 30) & 0b11;

                    // Shift pipeline forward
                    component.state.background.attribute_shift <<= 2;
                    component.state.background.pattern_low_shift <<= 1;
                    component.state.background.pattern_high_shift <<= 1;

                    let background_pixel = component.state.calculate_background_color::<R>(
                        &self.ppu_address_space,
                        attribute as u8,
                        color_index as u8,
                    );

                    let pixel = if component.state.oam.rendering_enabled {
                        sprite_pixel
                    } else {
                        None
                    }
                    .or(if component.state.background.rendering_enabled {
                        Some(background_pixel)
                    } else {
                        None
                    })
                    .unwrap_or(BLACK);

                    backend.modify_staging_buffer(|mut staging_buffer_guard| {
                        staging_buffer_guard[(
                            scanline_position_x as usize,
                            component.state.cycle_counter.y as usize,
                        )] = pixel.into();
                    });
                }

                if let 65..=256 = component.state.cycle_counter.x {
                    let sprite_index = (component.state.cycle_counter.x - 65) / 2;
                    let oam_data_index = sprite_index * 4;

                    if sprite_index < 64 {
                        match component.state.oam.sprite_evaluation_state {
                            SpriteEvaluationState::InspectingY => {
                                let sprite_y = component.state.oam.data[oam_data_index as usize];

                                component.state.oam.sprite_evaluation_state =
                                    SpriteEvaluationState::Evaluating { sprite_y };
                            }
                            SpriteEvaluationState::Evaluating { sprite_y } => {
                                if (u16::from(sprite_y)..u16::from(sprite_y) + 8)
                                    .contains(&(component.state.cycle_counter.y))
                                {
                                    let mut bytes = [0; 4];
                                    bytes.copy_from_slice(
                                        &component.state.oam.data
                                            [RangeInclusive::from_start_and_length(
                                                oam_data_index as usize,
                                                4,
                                            )],
                                    );

                                    let sprite = OamSprite::from_bytes(bytes);

                                    if component.state.oam.secondary_data.try_push(sprite).is_err()
                                    {
                                        // TODO: Handle sprite overflow flag
                                    }
                                }

                                component.state.oam.sprite_evaluation_state =
                                    SpriteEvaluationState::InspectingY;
                            }
                        }
                    }
                }

                if component.state.cycle_counter.x == 256
                    && component.state.background.rendering_enabled
                {
                    let mut vram_address_pointer_contents =
                        VramAddressPointerContents::from(component.state.vram_address_pointer);

                    if vram_address_pointer_contents.fine_y == 7 {
                        vram_address_pointer_contents.fine_y = 0;

                        if vram_address_pointer_contents.coarse.y == 29 {
                            vram_address_pointer_contents.coarse.y = 0;

                            vram_address_pointer_contents.nametable.y =
                                !vram_address_pointer_contents.nametable.y;
                        } else if vram_address_pointer_contents.coarse.y == 31 {
                            vram_address_pointer_contents.coarse.y = 0;
                        } else {
                            vram_address_pointer_contents.coarse.y += 1;
                        }
                    } else {
                        vram_address_pointer_contents.fine_y += 1;
                    }

                    component.state.vram_address_pointer = u16::from(vram_address_pointer_contents);
                }

                if component.state.cycle_counter.x == 257 {
                    component.state.oam.currently_rendering_sprites.clear();

                    if component.state.background.rendering_enabled {
                        let t = VramAddressPointerContents::from(
                            component.state.shadow_vram_address_pointer,
                        );
                        let mut v =
                            VramAddressPointerContents::from(component.state.vram_address_pointer);

                        v.nametable.x = t.nametable.x;
                        v.coarse.x = t.coarse.x;

                        component.state.vram_address_pointer = u16::from(v);
                    }
                }

                if let 257..=320 = component.state.cycle_counter.x {
                    component
                        .state
                        .drive_sprite_pipeline::<R>(&self.ppu_address_space);
                }

                if let 321..=336 = component.state.cycle_counter.x {
                    component
                        .state
                        .drive_background_pipeline::<R>(&self.ppu_address_space);
                }
            }

            component.state.cycle_counter = component.state.get_modified_cycle_counter::<R>(1);
        }
    }
}

impl State {
    fn drive_sprite_pipeline<R: Region>(&mut self, ppu_address_space: &AddressSpace) {
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
                            .read_le_value(address as usize, false)
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
                            .read_le_value(address as usize, false)
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

    fn drive_background_pipeline<R: Region>(&mut self, ppu_address_space: &AddressSpace) {
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

                    let nametable = ppu_address_space.read_le_value(address, false).unwrap();

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

                    let attribute: u8 = ppu_address_space.read_le_value(address, false).unwrap();

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
                        .read_le_value(address as usize, false)
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
                        .read_le_value(address as usize, false)
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

    #[inline]
    fn get_modified_cycle_counter<R: Region>(&self, amount: u16) -> Point2<u16> {
        let mut cycle_counter = self.cycle_counter;
        cycle_counter.x += amount;

        if cycle_counter.x > TOTAL_SCANLINE_LENGTH {
            cycle_counter.x = 0;
            cycle_counter.y += 1;
        }

        if cycle_counter.y > R::TOTAL_SCANLINES {
            cycle_counter.y = 0;
        }

        cycle_counter
    }

    // This function uses manual bit math because absolute speed is critical here

    #[inline]
    fn calculate_background_color<R: Region>(
        &self,
        ppu_address_space: &AddressSpace,
        attribute: u8,
        color: u8,
    ) -> Srgb<u8> {
        let color_bits = color & 0b11;

        // Combine into a 4-bit palette index
        let palette_index = color_bits | (attribute << 2);

        let color_value: u8 = ppu_address_space
            .read_le_value(
                BACKGROUND_PALETTE_BASE_ADDRESS + palette_index as usize,
                false,
            )
            .unwrap();

        let color = PpuColor {
            hue: color_value & 0b1111,
            luminance: (color_value >> 4) & 0b11,
        };

        R::color_to_srgb(color)
    }

    #[inline]
    fn calculate_sprite_color<R: Region>(
        &self,
        ppu_address_space: &AddressSpace,
        sprite: OamSprite,
        color: u8,
    ) -> Srgb<u8> {
        let color_bits = color & 0b11;

        let color_value: u8 = ppu_address_space
            .read_le_value(
                SPRITE_PALETTE_BASE_ADDRESS
                    + (usize::from(sprite.palette_index) * 4)
                    + usize::from(color_bits),
                false,
            )
            .unwrap();

        let color = PpuColor {
            hue: color_value & 0b1111,
            luminance: (color_value >> 4) & 0b11,
        };

        R::color_to_srgb(color)
    }
}
