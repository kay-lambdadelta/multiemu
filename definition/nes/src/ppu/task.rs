use crate::ppu::{
    BACKGROUND_PALETTE_BASE_ADDRESS, Ppu, State, TOTAL_SCANLINE_LENGTH,
    backend::{PpuDisplayBackend, SupportedGraphicsApiPpu},
    background::BackgroundPipelineState,
    color::PpuColor,
    oam::{OamSprite, SpriteEvaluationState},
    region::Region,
};
use multiemu_definition_mos6502::NmiFlag;
use multiemu_runtime::{
    memory::{AddressSpaceId, MemoryAccessTable},
    scheduler::Task,
};
use nalgebra::{Point2, Vector2};
use palette::Srgb;
use std::{
    num::NonZero,
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

        for _ in 0..time_slice.get() {
            if std::mem::replace(&mut component.state.reset_cpu_nmi, false) {
                self.processor_nmi.store(false);
            }

            if component.state.cycle_counter.x == 1 {
                // Technically the NES does it over 64 cycles
                component.state.oam.queued_sprites.clear();

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

                        backend.commit_staging_buffer();
                    }
                    _ => {}
                }
            }

            /*
            if (matches!(component.state.cycle_counter.x, 321..=336)
                && component.state.cycle_counter.y == 261)
            {
                component
                    .state
                    .drive_pipeline::<R>(&component.memory_access_table, self.ppu_address_space);
            }
            */

            if (0..R::VISIBLE_SCANLINES).contains(&component.state.cycle_counter.y) {
                if let 1..=256 = component.state.cycle_counter.x {
                    component.state.drive_background_pipeline::<R>(
                        &component.memory_access_table,
                        self.ppu_address_space,
                    );

                    // Extract out and combine pattern bits
                    let high = (component.state.background.pattern_high_shift >> 15) & 1;
                    let low = (component.state.background.pattern_low_shift >> 15) & 1;
                    let color = (high << 1) | low;

                    // Extract out attribute bits
                    let attribute = (component.state.background.attribute_shift >> 30) & 0b11;

                    // Shift pipeline forward
                    component.state.background.attribute_shift <<= 2;
                    component.state.background.pattern_low_shift <<= 1;
                    component.state.background.pattern_high_shift <<= 1;

                    let color = component.state.calculate_color::<R>(
                        &component.memory_access_table,
                        self.ppu_address_space,
                        attribute as u8,
                        color as u8,
                    );

                    backend.modify_staging_buffer(|mut staging_buffer_guard| {
                        staging_buffer_guard[(
                            component.state.cycle_counter.x as usize - 1,
                            component.state.cycle_counter.y as usize,
                        )] = color.into();
                    });
                }

                if let 65..=256 = component.state.cycle_counter.x {
                    let oam_data_index = component.state.cycle_counter.x - 65;

                    match component.state.oam.sprite_evaluation_state {
                        SpriteEvaluationState::InspectingY => {
                            let sprite_y = component.state.oam.data[oam_data_index as usize];

                            component.state.oam.sprite_evaluation_state =
                                SpriteEvaluationState::Evaluating { sprite_y };
                        }
                        SpriteEvaluationState::Evaluating { sprite_y } => {
                            if sprite_y as u16 == component.state.cycle_counter.y + 1 {
                                let mut bytes = [0; 4];

                                #[allow(clippy::needless_range_loop)]
                                for i in 0..4 {
                                    bytes[i] =
                                        component.state.oam.data[oam_data_index as usize + i];
                                }

                                let sprite = OamSprite::from_bytes(bytes);

                                if component.state.oam.queued_sprites.try_push(sprite).is_err() {
                                    // TODO: Handle sprite overflow flag
                                }

                                component.state.oam.sprite_evaluation_state =
                                    SpriteEvaluationState::InspectingY;
                            }
                        }
                    }
                }
            }

            component.state.cycle_counter = component.state.get_modified_cycle_counter::<R>(1);
        }
    }
}

impl State {
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
                    let nametable = self.fetch_nametable::<R>(access_table, ppu_address_space);

                    self.background_pipeline_state =
                        BackgroundPipelineState::FetchingAttribute { nametable };
                }
                BackgroundPipelineState::FetchingAttribute { nametable } => {
                    let attribute = self.fetch_attribute::<R>(access_table, ppu_address_space);

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
                    let pattern_table_low = self.fetch_pattern_table_low::<R>(
                        access_table,
                        ppu_address_space,
                        nametable,
                    );

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
                    let pattern_table_high = self.fetch_pattern_table_high::<R>(
                        access_table,
                        ppu_address_space,
                        nametable,
                    );

                    self.background.pattern_low_shift =
                        (self.background.pattern_low_shift & 0xff00) | pattern_table_low as u16;
                    self.background.pattern_high_shift =
                        (self.background.pattern_high_shift & 0xff00) | pattern_table_high as u16;
                    self.background.attribute_shift = (self.background.attribute_shift
                        & 0xffff0000)
                        | (attribute as u32 * 0x5555);

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

    #[inline]
    fn fetch_nametable<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
    ) -> u8 {
        let scrolled = self.scrolled();

        let nametable = scrolled.component_div(&Vector2::new(256, 240));
        let nametable_index = nametable.x + nametable.y * 2;
        let base_address = self.nametable_base + nametable_index * 0x400;
        let tile_position = self.tile_position();

        let address = base_address + tile_position.y * 32 + tile_position.x;

        memory_access_table
            .read_le_value(address as usize, ppu_address_space, false)
            .unwrap()
    }

    #[inline]
    fn fetch_attribute<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
    ) -> u8 {
        let tile_position = self.tile_position();
        let attribute_position = tile_position / 4;

        let attribute_base = self.nametable_base + 0x3c0;
        let address = attribute_base + attribute_position.y * 8 + attribute_position.x;

        let attribute: u8 = memory_access_table
            .read_le_value(address as usize, ppu_address_space, false)
            .unwrap();

        let attribute_quadrant = Point2::new(tile_position.x % 4, tile_position.y % 4) / 2;
        let shift = (attribute_quadrant.y * 2 + attribute_quadrant.x) * 2;

        (attribute >> shift) & 0b11
    }

    #[inline]
    fn fetch_pattern_table_low<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
        nametable: u8,
    ) -> u8 {
        let row = self.scrolled().y % 8;
        let address = self.background.pattern_table_base + (nametable as u16) * 16 + row;

        memory_access_table
            .read_le_value(address as usize, ppu_address_space, false)
            .unwrap()
    }

    #[inline]
    fn fetch_pattern_table_high<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
        nametable: u8,
    ) -> u8 {
        let row = self.scrolled().y % 8;
        let address = self.background.pattern_table_base + (nametable as u16) * 16 + row + 8;

        memory_access_table
            .read_le_value(address as usize, ppu_address_space, false)
            .unwrap()
    }

    // This function uses manual bit math because absolute speed is critical here

    #[inline]
    fn calculate_color<R: Region>(
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
                BACKGROUND_PALETTE_BASE_ADDRESS + palette_index as usize,
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
