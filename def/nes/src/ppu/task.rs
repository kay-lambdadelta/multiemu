use crate::ppu::{
    BACKGROUND_PALETTE_BASE_ADDRESS, DrawingState, Ppu, State, TOTAL_SCANLINE_LENGTH,
    VISIBLE_SCANLINE_LENGTH,
    backend::{PpuDisplayBackend, SupportedGraphicsApiPpu},
    color::PpuColor,
    region::Region,
};
use bitvec::{field::BitField, view::BitView};
use deku::bitvec::{BitArray, Lsb0, Msb0};
use multiemu::{
    memory::{AddressSpaceId, MemoryAccessTable},
    scheduler::TaskMut,
};
use multiemu_definition_mos6502::NmiFlag;
use nalgebra::Point2;
use palette::Srgb;
use std::{
    num::NonZero,
    sync::{Arc, atomic::Ordering},
};

pub struct Driver {
    pub processor_nmi: Arc<NmiFlag>,
    pub memory_access_table: Arc<MemoryAccessTable>,
    pub ppu_address_space: AddressSpaceId,
}

const FINAL_VISIBLE_CYCLE: u16 = VISIBLE_SCANLINE_LENGTH;

impl<R: Region, G: SupportedGraphicsApiPpu> TaskMut<Ppu<R, G>> for Driver {
    fn run(&mut self, component: &mut Ppu<R, G>, time_slice: NonZero<u32>) {
        let backend = component.backend.get_mut().unwrap();

        for _ in 0..time_slice.get() {
            if std::mem::replace(&mut component.state.reset_cpu_nmi, false) {
                self.processor_nmi.store(false);
            }

            if component.state.cycle_counter.x == 0 {
                // Do nothing

                // Use this to present frame
                if component.state.cycle_counter.y == 0 {
                    backend.commit_staging_buffer();
                }
            } else if (0..R::VISIBLE_SCANLINES).contains(&component.state.cycle_counter.y) {
                self.handle_visible_cycles::<R, G>(&mut component.state, backend);
            } else if component.state.cycle_counter.y == 241 {
                component
                    .state
                    .entered_vblank
                    .store(true, Ordering::Release);

                if component.state.vblank_nmi_enabled {
                    self.processor_nmi.store(true);
                    component.state.reset_cpu_nmi = true;
                }
            }

            component.state.cycle_counter = component.state.get_modified_cycle_counter::<R>(1);
        }
    }
}

impl Driver {
    #[inline]
    fn handle_visible_cycles<R: Region, G: SupportedGraphicsApiPpu>(
        &self,
        state: &mut State,
        backend: &mut <G as SupportedGraphicsApiPpu>::Backend<R>,
    ) {
        match state.cycle_counter.x {
            1..=FINAL_VISIBLE_CYCLE => {
                if state.cycle_counter.x == 1 {
                    state.entered_vblank.store(false, Ordering::Release);
                }

                // Steps wait a cycle inbetween for memory access realism
                if !state.awaiting_memory_access {
                    // Swap out the pipeline state with a placeholder for a moment
                    match std::mem::replace(
                        &mut state.drawing_state,
                        DrawingState::FetchingNametable,
                    ) {
                        DrawingState::FetchingNametable => {
                            let nametable = state.fetch_nametable::<R>(
                                &self.memory_access_table,
                                self.ppu_address_space,
                            );

                            state.drawing_state = DrawingState::FetchingAttribute { nametable };
                        }
                        DrawingState::FetchingAttribute { nametable } => {
                            let attribute = state.fetch_attribute::<R>(
                                &self.memory_access_table,
                                self.ppu_address_space,
                            );

                            // TODO: Placeholder
                            state.drawing_state = DrawingState::FetchingPatternTableLow {
                                nametable,
                                attribute,
                            };
                        }
                        DrawingState::FetchingPatternTableLow {
                            nametable,
                            attribute,
                        } => {
                            let pattern_table_low = state.fetch_pattern_table_low::<R>(
                                &self.memory_access_table,
                                self.ppu_address_space,
                                nametable,
                            );

                            state.drawing_state = DrawingState::FetchingPatternTableHigh {
                                nametable,
                                attribute,
                                pattern_table_low,
                            };
                        }
                        DrawingState::FetchingPatternTableHigh {
                            nametable,
                            attribute,
                            pattern_table_low,
                        } => {
                            let pattern_table_high = state.fetch_pattern_table_high::<R>(
                                &self.memory_access_table,
                                self.ppu_address_space,
                                nametable,
                            );

                            let pattern_low_bits = pattern_table_low.view_bits::<Msb0>();
                            let pattern_high_bits = pattern_table_high.view_bits::<Msb0>();

                            for (low, high) in pattern_low_bits.into_iter().zip(pattern_high_bits) {
                                let mut color_bits: BitArray<u8, Lsb0> = Default::default();
                                color_bits.set(0, *low);
                                color_bits.set(1, *high);

                                let color = color_bits.load::<u8>();

                                let color = state.calculate_color::<R>(
                                    &self.memory_access_table,
                                    self.ppu_address_space,
                                    attribute,
                                    color,
                                );

                                // TODO: this just renders in greyscale
                                state.pixel_queue.push_back(color.into());
                            }
                        }
                    }
                }

                let color = state.pixel_queue.pop_front().expect("Pixel queue ran dry");

                backend.modify_staging_buffer(|mut staging_buffer_guard| {
                    staging_buffer_guard[(
                        // Offset for idle cycle
                        state.cycle_counter.x as usize - 1,
                        state.cycle_counter.y as usize,
                    )] = color.into();
                });

                state.awaiting_memory_access = !state.awaiting_memory_access;
            }

            _ => {}
        }
    }
}

impl State {
    #[inline]
    fn get_modified_cycle_counter<R: Region>(&self, amount: u16) -> Point2<u16> {
        let mut cycle_counter = self.cycle_counter;
        cycle_counter.x += amount;

        if cycle_counter.x >= TOTAL_SCANLINE_LENGTH {
            cycle_counter.x -= TOTAL_SCANLINE_LENGTH;
            cycle_counter.y += 1;
        }

        if cycle_counter.y >= R::TOTAL_SCANLINES {
            cycle_counter.y -= R::TOTAL_SCANLINES;
        }

        cycle_counter
    }

    #[inline]
    fn fetch_nametable<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
    ) -> u8 {
        let tile_position = self.get_modified_cycle_counter::<R>(8) / 8;

        let address = self.nametable_base + tile_position.y * 32 + tile_position.x;

        memory_access_table
            .read_le_value(address as usize, ppu_address_space)
            .unwrap()
    }

    #[inline]
    fn fetch_attribute<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
    ) -> u8 {
        let tile_position = self.get_modified_cycle_counter::<R>(8) / 8;
        let attribute_position = tile_position / 4;

        let attribute_base = self.nametable_base + 0x3C0;
        let address = attribute_base + attribute_position.y * 8 + attribute_position.x;

        memory_access_table
            .read_le_value(address as usize, ppu_address_space)
            .unwrap()
    }

    #[inline]
    fn fetch_pattern_table_low<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
        nametable: u8,
    ) -> u8 {
        let cycle_counter = self.get_modified_cycle_counter::<R>(8);

        let row = cycle_counter.y % 8;
        let address = self.background_pattern_table_base + (nametable as u16) * 16 + row;

        memory_access_table
            .read_le_value(address as usize, ppu_address_space)
            .unwrap()
    }

    #[inline]
    fn fetch_pattern_table_high<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
        nametable: u8,
    ) -> u8 {
        let cycle_counter = self.get_modified_cycle_counter::<R>(8);

        let row = cycle_counter.y % 8;
        let address = self.background_pattern_table_base + (nametable as u16) * 16 + row + 8;

        memory_access_table
            .read_le_value(address as usize, ppu_address_space)
            .unwrap()
    }

    // This function uses manual bit math because absolute speed is critical here

    #[inline]
    fn calculate_color<R: Region>(
        &self,
        memory_access_table: &MemoryAccessTable,
        ppu_address_space: AddressSpaceId,
        attribute_byte: u8,
        color: u8,
    ) -> Srgb<u8> {
        let tile_position = self.get_modified_cycle_counter::<R>(8) / 8;

        let quadrant = Point2::new(tile_position.x % 4, tile_position.y % 4) / 2;
        let shift = (quadrant.y * 2 + quadrant.x) * 2;

        let color_bits = color & 0b11; // lowest 2 bits
        let attribute_bits = (attribute_byte >> shift) & 0b11; // 2 bits from attribute byte

        // Combine into a 4-bit palette index
        let palette_index = color_bits | (attribute_bits << 2);

        let color_value: u8 = memory_access_table
            .read_le_value(
                BACKGROUND_PALETTE_BASE_ADDRESS + palette_index as usize,
                ppu_address_space,
            )
            .unwrap();

        let color = PpuColor {
            hue: color_value & 0b1111,
            luminance: (color_value >> 4) & 0b11,
        };

        R::color_to_srgb(color)
    }
}
