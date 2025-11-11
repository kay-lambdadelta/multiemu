use crate::ppu::{
    PALETTE_BASE_ADDRESS, Ppu, State, TOTAL_SCANLINE_LENGTH,
    backend::{PpuDisplayBackend, SupportedGraphicsApiPpu},
    background::{BackgroundPipelineState, SpritePipelineState},
    color::PpuColor,
    oam::{CurrentlyRenderingSprite, OamSprite, SpriteEvaluationState},
    region::Region,
};
use multiemu_definition_mos6502::NmiFlag;
use multiemu_range::ContiguousRange;
use multiemu_runtime::{
    memory::{AddressSpaceId, MemoryAccessTable},
    scheduler::Task,
};
use nalgebra::{Point2, Vector2};
use palette::{Srgb, named::BLACK};
use std::{
    num::NonZero,
    ops::RangeInclusive,
    sync::{Arc, atomic::Ordering},
};

const PIPELINE_PREFETCH: u16 = 16;

pub struct Driver {
    pub processor_nmi: Arc<NmiFlag>,
    pub ppu_address_space: AddressSpaceId,
}

impl<R: Region, G: SupportedGraphicsApiPpu> Task<Ppu<R, G>> for Driver {
    fn run(&mut self, component: &mut Ppu<R, G>, time_slice: NonZero<u32>) {
        let backend = component.backend.as_mut().unwrap();
        let mut commit_staging_buffer = false;

        for _ in 0..time_slice.get() {
            if std::mem::replace(&mut component.state.reset_cpu_nmi, false) {
                self.processor_nmi.store(false);
            }

            if component.state.cycle_counter.x == 1 {
                match component.state.cycle_counter.y {
                    241 => {
                        component
                            .state
                            .entered_vblank
                            .store(true, Ordering::Release);

                        if component.state.vblank_nmi_enabled {
                            self.processor_nmi.store(true);
                            component.state.reset_cpu_nmi = true;
                        }
                    }
                    261 => {
                        component
                            .state
                            .entered_vblank
                            .store(false, Ordering::Release);

                        commit_staging_buffer = true;
                    }
                    _ => {}
                }
            }

            if (0..R::VISIBLE_SCANLINES).contains(&component.state.cycle_counter.y) {
                if component.state.cycle_counter.x == 1 {
                    // Technically the NES does it over 64 cycles
                    component.state.oam.secondary_data.clear();
                }

                if let 1..=256 = component.state.cycle_counter.x {
                    let scanline_position_x = component.state.cycle_counter.x - 1;

                    component.state.drive_background_pipeline::<R>(
                        &component.memory_access_table,
                        self.ppu_address_space,
                    );

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
                            &component.memory_access_table,
                            self.ppu_address_space,
                            &sprite.oam,
                            color_index,
                        ));
                    }

                    // Extract out and combine pattern bits
                    let high = (component.state.background.pattern_high_shift >> 15) & 1;
                    let low = (component.state.background.pattern_low_shift >> 15) & 1;
                    let color_index = (high << 1) | low;

                    // Extract out attribute bits
                    let attribute = (component.state.background.attribute_shift >> 30) & 0b11;

                    // Shift pipeline forward
                    component.state.background.attribute_shift <<= 2;
                    component.state.background.pattern_low_shift <<= 1;
                    component.state.background.pattern_high_shift <<= 1;

                    let background_pixel = component.state.calculate_background_color::<R>(
                        &component.memory_access_table,
                        self.ppu_address_space,
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

                if component.state.cycle_counter.x == 257 {
                    component.state.oam.currently_rendering_sprites.clear();
                }

                if let 257..=320 = component.state.cycle_counter.x {
                    component.state.drive_sprite_pipeline::<R>(
                        &component.memory_access_table,
                        self.ppu_address_space,
                    );
                }
            }

            component.state.cycle_counter = component.state.get_modified_cycle_counter::<R>(1);
        }

        if commit_staging_buffer {
            backend.commit_staging_buffer();
        }
    }
}

impl State {
    fn drive_sprite_pipeline<R: Region>(
        &mut self,
        access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
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

                        let address = self.oam.sprite_8x8_pattern_table_address
                            + u16::from(currently_relevant_sprite.tile_index) * 16
                            + row;

                        let pattern_table_low = access_table
                            .read_le_value(address as usize, ppu_address_space, false)
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

                        let address = self.oam.sprite_8x8_pattern_table_address
                            + u16::from(currently_relevant_sprite.tile_index) * 16
                            + row
                            + 8;

                        let pattern_table_high = access_table
                            .read_le_value(address as usize, ppu_address_space, false)
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

    fn drive_background_pipeline<R: Region>(
        &mut self,
        access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
    ) {
        // Steps wait a cycle inbetween for memory access realism
        if !self.awaiting_memory_access {
            // Swap out the pipeline state with a placeholder for a moment
            match self.background_pipeline_state {
                BackgroundPipelineState::FetchingNametable => {
                    let scrolled = self.scrolled();
                    let nametable = scrolled.component_div(&Vector2::new(256, 240));
                    let nametable_index = nametable.x + nametable.y * 2;
                    let base_address = self.nametable_base + nametable_index * 0x400;
                    let tile_position = self.tile_position();

                    let address = base_address + tile_position.y * 32 + tile_position.x;

                    let nametable = access_table
                        .read_le_value(address as usize, ppu_address_space, false)
                        .unwrap();

                    self.background_pipeline_state =
                        BackgroundPipelineState::FetchingAttribute { nametable };
                }
                BackgroundPipelineState::FetchingAttribute { nametable } => {
                    let tile_position = self.tile_position();
                    let attribute_position = tile_position / 4;

                    let attribute_base = self.nametable_base + 0x3c0;
                    let address = attribute_base + attribute_position.y * 8 + attribute_position.x;

                    let attribute: u8 = access_table
                        .read_le_value(address as usize, ppu_address_space, false)
                        .unwrap();

                    let attribute_quadrant =
                        Point2::new(tile_position.x % 4, tile_position.y % 4) / 2;
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
                    let row = self.scrolled().y % 8;
                    let address =
                        self.background.pattern_table_base + u16::from(nametable) * 16 + row;

                    let pattern_table_low = access_table
                        .read_le_value(address as usize, ppu_address_space, false)
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
                    let row = self.scrolled().y % 8;
                    let address =
                        self.background.pattern_table_base + u16::from(nametable) * 16 + row + 8;

                    let pattern_table_high: u8 = access_table
                        .read_le_value(address as usize, ppu_address_space, false)
                        .unwrap();

                    self.background.pattern_low_shift =
                        (self.background.pattern_low_shift & 0xff00) | u16::from(pattern_table_low);
                    self.background.pattern_high_shift = (self.background.pattern_high_shift
                        & 0xff00)
                        | u16::from(pattern_table_high);
                    self.background.attribute_shift = (self.background.attribute_shift
                        & 0xffff0000)
                        // Spread the bits
                        | (u32::from(attribute) * 0x5555);

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
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
        attribute: u8,
        color: u8,
    ) -> Srgb<u8> {
        let color_bits = color & 0b11;

        // Combine into a 4-bit palette index
        let palette_index = color_bits | (attribute << 2);

        let color_value: u8 = memory_access_table
            .read_le_value(
                PALETTE_BASE_ADDRESS + palette_index as usize,
                ppu_address_space,
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
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
        sprite: &OamSprite,
        color: u8,
    ) -> Srgb<u8> {
        let color_bits = color & 0b11;

        let color_value: u8 = memory_access_table
            .read_le_value(
                PALETTE_BASE_ADDRESS
                    + 0x10
                    + (usize::from(sprite.palette_index) * 4)
                    + usize::from(color_bits),
                ppu_address_space,
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
    pub fn scrolled(&self) -> Vector2<u16> {
        self.background.coarse_scroll.cast() * 8
            + self.background.fine_scroll.cast()
            + Vector2::new(
                self.cycle_counter.x - 1 + PIPELINE_PREFETCH,
                self.cycle_counter.y,
            )
    }

    #[inline]
    pub fn tile_position(&self) -> Point2<u16> {
        let scrolled = self.scrolled();

        let tile_position = Vector2::new(scrolled.x % 256, scrolled.y % 240) / 8;

        tile_position.into()
    }
}
