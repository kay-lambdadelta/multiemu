use std::{
    any::Any,
    marker::PhantomData,
    ops::RangeInclusive,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
};

use arrayvec::ArrayVec;
use bitvec::{array::BitArray, field::BitField, prelude::Lsb0, view::BitView};
use fluxemu_definition_mos6502::{Mos6502, NmiFlag, RdyFlag};
use fluxemu_range::ContiguousRange;
use fluxemu_runtime::{
    component::{Component, ComponentConfig, LateInitializedData},
    machine::{
        Machine,
        builder::{ComponentBuilder, SchedulerParticipation},
    },
    memory::{Address, AddressSpace, AddressSpaceCache, AddressSpaceId, MemoryError},
    path::FluxEmuPath,
    platform::Platform,
    scheduler::{Period, SynchronizationContext},
};
use nalgebra::{Point2, Vector2};
use palette::named::BLACK;
use serde::{Deserialize, Serialize};
use strum::FromRepr;

use crate::ppu::{
    backend::{PpuDisplayBackend, SupportedGraphicsApiPpu},
    background::{BackgroundPipelineState, BackgroundState, SpritePipelineState},
    oam::{OamSprite, OamState, SpriteEvaluationState},
    region::Region,
    state::{State, VramAddressPointerContents},
};

pub mod backend;
mod background;
mod color;
mod oam;
pub mod region;
mod state;

#[derive(Clone, Copy, Debug, FromRepr)]
#[repr(u16)]
pub enum CpuAccessibleRegister {
    PpuCtrl = 0x2000,
    PpuMask = 0x2001,
    PpuStatus = 0x2002,
    OamAddr = 0x2003,
    OamData = 0x2004,
    PpuScroll = 0x2005,
    PpuAddr = 0x2006,
    PpuData = 0x2007,
    OamDma = 0x4014,
}

pub const NAMETABLE_ADDRESSES: [RangeInclusive<Address>; 4] = [
    0x2000..=0x23ff,
    0x2400..=0x27ff,
    0x2800..=0x2bff,
    0x2c00..=0x2fff,
];
pub const NAMETABLE_BASE_ADDRESS: Address = *NAMETABLE_ADDRESSES[0].start();
pub const BACKGROUND_PALETTE_BASE_ADDRESS: Address = 0x3f00;
pub const SPRITE_PALETTE_BASE_ADDRESS: Address = 0x3f10;
pub const ATTRIBUTE_BASE_ADDRESS: Address = NAMETABLE_BASE_ADDRESS + 0x3c0;
const DUMMY_SCANLINE_COUNT: u16 = 2;
const VISIBLE_SCANLINE_LENGTH: u16 = 256;
const HBLANK_LENGTH: u16 = 85;
const TOTAL_SCANLINE_LENGTH: u16 = VISIBLE_SCANLINE_LENGTH + HBLANK_LENGTH;
const INITIAL_CYCLE_COUNTER_POSITION: Point2<u16> = Point2::new(0, 0);

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ColorEmphasis {
    /// This actually means green on pal/dendy
    pub red: bool,
    /// This actually means red on pal/dendy
    pub green: bool,
    pub blue: bool,
}

#[derive(Debug)]
pub struct PpuConfig<R: Region> {
    pub cpu_address_space: AddressSpaceId,
    pub ppu_address_space: AddressSpaceId,
    pub processor: FluxEmuPath,
    pub _phantom: PhantomData<R>,
}

#[derive(Debug)]
pub struct Ppu<R: Region, G: SupportedGraphicsApiPpu> {
    state: State,
    backend: Option<G::Backend<R>>,
    cpu_address_space: Arc<AddressSpace>,
    ppu_address_space: Arc<AddressSpace>,
    processor_rdy: Arc<RdyFlag>,
    processor_nmi: Arc<NmiFlag>,
    ppu_address_space_cache: AddressSpaceCache,
    my_path: FluxEmuPath,
    machine: Weak<Machine>,
    timestamp: Period,
    period: Period,
}

impl<R: Region, P: Platform<GraphicsApi: SupportedGraphicsApiPpu>> ComponentConfig<P>
    for PpuConfig<R>
{
    type Component = Ppu<R, P::GraphicsApi>;

    fn late_initialize(component: &mut Self::Component, data: &LateInitializedData<P>) {
        component.backend = Some(PpuDisplayBackend::new(
            data.component_graphics_initialization_data.clone(),
        ));

        component.machine = data.machine.clone();
    }

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let frequency = R::master_clock() / 4;

        let ppu_address_space = component_builder
            .get_address_space(self.ppu_address_space)
            .clone();
        let cpu_address_space = component_builder
            .get_address_space(self.cpu_address_space)
            .clone();
        let my_path = component_builder.path().clone();

        let (component_builder, _) = component_builder
            .set_scheduler_participation(SchedulerParticipation::OnDemand)
            .insert_display("tv");

        let processor_nmi = component_builder
            .interact::<Mos6502, _>(&self.processor, Mos6502::nmi)
            .unwrap();

        // Install the rdy source
        let processor_rdy = component_builder
            .interact::<Mos6502, _>(&self.processor, Mos6502::rdy)
            .unwrap();

        let total_screen_time =
            Period::from_num(TOTAL_SCANLINE_LENGTH as u32 * R::TOTAL_SCANLINES as u32) / frequency;
        let framerate = total_screen_time.recip();

        let vblank_start_from_initial_position =
            (Period::from_num(TOTAL_SCANLINE_LENGTH) * 241 + Period::from_num(1)) / frequency;

        let vblank_end_from_initial_position =
            (Period::from_num(TOTAL_SCANLINE_LENGTH) * 261 + Period::from_num(1)) / frequency;

        component_builder
            .memory_map_component_write(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuCtrl as usize..=CpuAccessibleRegister::PpuCtrl as usize,
            )
            .memory_map_component_write(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuScroll as usize
                    ..=CpuAccessibleRegister::PpuScroll as usize,
            )
            .memory_map_component_write(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuMask as usize..=CpuAccessibleRegister::PpuMask as usize,
            )
            .memory_map_component_read(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuStatus as usize
                    ..=CpuAccessibleRegister::PpuStatus as usize,
            )
            .memory_map_component(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuAddr as usize..=CpuAccessibleRegister::PpuAddr as usize,
            )
            .memory_map_component(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuData as usize..=CpuAccessibleRegister::PpuData as usize,
            )
            .memory_map_component(
                self.cpu_address_space,
                CpuAccessibleRegister::OamAddr as usize..=CpuAccessibleRegister::OamAddr as usize,
            )
            .memory_map_component(
                self.cpu_address_space,
                CpuAccessibleRegister::OamData as usize..=CpuAccessibleRegister::OamData as usize,
            )
            .memory_map_component_write(
                self.cpu_address_space,
                CpuAccessibleRegister::OamDma as usize..=CpuAccessibleRegister::OamDma as usize,
            )
            .schedule_repeating_event(
                // x: 1, y: 241
                vblank_start_from_initial_position,
                framerate,
                Ppu::vblank_start,
            )
            .schedule_repeating_event(
                // x: 1, y: 261
                vblank_end_from_initial_position,
                framerate,
                Ppu::vblank_end,
            );

        Ok(Ppu {
            state: State {
                sprite_size: Vector2::new(8, 8),
                vblank_nmi_enabled: false,
                greyscale: false,
                entered_vblank: AtomicBool::new(false),
                show_background_leftmost_pixels: false,
                vram_address_pointer_write_phase: false,
                vram_address_pointer_increment_amount: 1,
                vram_read_buffer: AtomicU8::new(0),
                color_emphasis: ColorEmphasis {
                    red: false,
                    green: false,
                    blue: false,
                },
                cycle_counter: INITIAL_CYCLE_COUNTER_POSITION,
                awaiting_memory_access: true,
                background_pipeline_state: BackgroundPipelineState::FetchingNametable,
                sprite_pipeline_state: SpritePipelineState::FetchingNametableGarbage0,
                oam: OamState {
                    data: rand::random(),
                    oam_addr: 0x00,
                    sprite_evaluation_state: SpriteEvaluationState::InspectingY,
                    secondary_data: ArrayVec::new(),
                    currently_rendering_sprites: ArrayVec::new(),
                    show_sprites_leftmost_pixels: true,
                    sprite_8x8_pattern_table_index: 0x0000,
                    rendering_enabled: false,
                },
                background: BackgroundState {
                    pattern_table_index: 0x0000,
                    pattern_low_shift: 0,
                    pattern_high_shift: 0,
                    attribute_shift: 0,
                    fine_x_scroll: 0,
                    rendering_enabled: false,
                },
                vram_address_pointer: 0,
                shadow_vram_address_pointer: 0,
            },
            backend: None,
            cpu_address_space,
            processor_rdy,
            processor_nmi,
            ppu_address_space_cache: ppu_address_space.cache(),
            ppu_address_space,
            my_path,
            machine: Weak::new(),
            timestamp: Period::default(),
            period: frequency.recip(),
        })
    }
}

impl<R: Region, G: SupportedGraphicsApiPpu> Ppu<R, G> {
    fn vblank_start(&mut self, _timestamp: Period) {
        self.state.entered_vblank.store(true, Ordering::Release);

        if self.state.vblank_nmi_enabled {
            self.processor_nmi.store(false);
        }
    }

    fn vblank_end(&mut self, _timestamp: Period) {
        // Handle vblank nmi and then present the frame
        self.state.entered_vblank.store(false, Ordering::Release);
        self.processor_nmi.store(true);
        self.backend.as_mut().unwrap().commit_staging_buffer();
    }
}

impl<R: Region, G: SupportedGraphicsApiPpu> Component for Ppu<R, G> {
    fn memory_read(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        for (address, buffer) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter_mut())
        {
            let register = CpuAccessibleRegister::from_repr(address as u16).unwrap();
            tracing::debug!("Reading from PPU register: {:?}", register);

            match register {
                CpuAccessibleRegister::PpuMask => todo!(),
                CpuAccessibleRegister::PpuStatus => {
                    let buffer_bits = buffer.view_bits_mut::<Lsb0>();

                    // Currently in vblank
                    if avoid_side_effects {
                        buffer_bits.set(7, self.state.entered_vblank.load(Ordering::Acquire));
                    } else {
                        buffer_bits.set(7, self.state.entered_vblank.swap(false, Ordering::AcqRel));
                    }
                }
                CpuAccessibleRegister::OamAddr => {
                    *buffer = self.state.oam.oam_addr;
                }
                CpuAccessibleRegister::OamData => {
                    *buffer = self.state.oam.data[self.state.oam.oam_addr as usize];
                }
                CpuAccessibleRegister::PpuScroll => todo!(),
                CpuAccessibleRegister::PpuAddr => todo!(),
                CpuAccessibleRegister::PpuData => {
                    if avoid_side_effects {
                        *buffer = self.ppu_address_space.read_le_value_pure(
                            self.state.vram_address_pointer as usize,
                            self.timestamp,
                            None,
                        )?;
                    } else {
                        let new_value = self.ppu_address_space.read_le_value::<u8>(
                            self.state.vram_address_pointer as usize,
                            self.timestamp,
                            None,
                        )?;

                        *buffer = self
                            .state
                            .vram_read_buffer
                            .swap(new_value, Ordering::AcqRel);
                    }
                }
                _ => {
                    unreachable!("{:?}", register);
                }
            }
        }

        Ok(())
    }

    fn memory_write(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        for (address, buffer) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter())
        {
            let register = CpuAccessibleRegister::from_repr(address as u16).unwrap();
            tracing::debug!("Writing to PPU register: {:?}", register);

            match register {
                CpuAccessibleRegister::PpuCtrl => {
                    let data_bits = buffer.view_bits::<Lsb0>();

                    let mut shadow_vram_address_pointer =
                        VramAddressPointerContents::from(self.state.shadow_vram_address_pointer);

                    shadow_vram_address_pointer.nametable.x = data_bits[0];
                    shadow_vram_address_pointer.nametable.y = data_bits[1];

                    self.state.vram_address_pointer_increment_amount =
                        if data_bits[2] { 32 } else { 1 };

                    self.state.oam.sprite_8x8_pattern_table_index = u8::from(data_bits[3]);
                    self.state.background.pattern_table_index = u8::from(data_bits[4]);

                    self.state.vblank_nmi_enabled = data_bits[7];

                    self.state.shadow_vram_address_pointer = shadow_vram_address_pointer.into();
                }
                CpuAccessibleRegister::PpuMask => {
                    let data_bits = buffer.view_bits::<Lsb0>();

                    self.state.greyscale = data_bits[0];

                    self.state.show_background_leftmost_pixels = data_bits[1];
                    self.state.oam.show_sprites_leftmost_pixels = data_bits[2];

                    self.state.background.rendering_enabled = data_bits[3];
                    self.state.oam.rendering_enabled = data_bits[4];

                    self.state.color_emphasis.red = data_bits[5];
                    self.state.color_emphasis.green = data_bits[6];
                    self.state.color_emphasis.blue = data_bits[7];
                }
                CpuAccessibleRegister::OamAddr => {
                    self.state.oam.oam_addr = *buffer;
                }
                CpuAccessibleRegister::OamData => {
                    self.state.oam.data[self.state.oam.oam_addr as usize] = *buffer;
                    self.state.oam.oam_addr = self.state.oam.oam_addr.wrapping_add(1);
                }
                CpuAccessibleRegister::PpuScroll => {
                    // Convert the byte into a bit slice
                    let data_bits = BitArray::<_, Lsb0>::from(u16::from(*buffer));
                    let mut shadow_vram_address_pointer =
                        VramAddressPointerContents::from(self.state.shadow_vram_address_pointer);

                    if self.state.vram_address_pointer_write_phase {
                        // fine scroll y
                        shadow_vram_address_pointer.fine_y = data_bits[0..=2].load();
                        // coarse scroll y
                        shadow_vram_address_pointer.coarse.y = data_bits[3..=7].load();
                    } else {
                        // fine scroll x
                        self.state.background.fine_x_scroll = data_bits[0..=2].load();
                        // coarse scroll x
                        shadow_vram_address_pointer.coarse.x = data_bits[3..=7].load();
                    }

                    self.state.vram_address_pointer_write_phase =
                        !self.state.vram_address_pointer_write_phase;

                    self.state.shadow_vram_address_pointer = shadow_vram_address_pointer.into();
                }
                CpuAccessibleRegister::PpuAddr => {
                    let mut unpacked_address = self.state.shadow_vram_address_pointer.to_be_bytes();
                    unpacked_address[usize::from(self.state.vram_address_pointer_write_phase)] =
                        *buffer;
                    self.state.shadow_vram_address_pointer =
                        u16::from_be_bytes(unpacked_address) & 0b0111_1111_1111_1111;

                    self.state.vram_address_pointer_write_phase =
                        !self.state.vram_address_pointer_write_phase;

                    // Write the completed address
                    if !self.state.vram_address_pointer_write_phase {
                        self.state.vram_address_pointer = self.state.shadow_vram_address_pointer;
                    }
                }
                CpuAccessibleRegister::PpuData => {
                    tracing::debug!(
                        "CPU is sending data to 0x{:04x} in the PPU address space: {:02x}, the \
                         cycle counter is at {}",
                        self.state.vram_address_pointer,
                        buffer,
                        self.state.cycle_counter
                    );

                    // Redirect into the ppu address space
                    self.ppu_address_space.write_le_value(
                        self.state.vram_address_pointer as usize,
                        self.timestamp,
                        None,
                        *buffer,
                    )?;

                    self.state.vram_address_pointer =
                        self.state.vram_address_pointer.wrapping_add(u16::from(
                            self.state.vram_address_pointer_increment_amount,
                        )) & 0b0111_1111_1111_1111;
                }
                CpuAccessibleRegister::OamDma => {
                    let page = u16::from(*buffer) << 8;

                    self.processor_rdy.store(false);

                    // TODO: Extract to constant or extract from cpu directly within the config builder
                    let processor_frequency = R::master_clock() / 12;

                    // Make sure the cpu wakes up eventually
                    self.machine.upgrade().unwrap().schedule_event::<Self>(
                        self.timestamp + (processor_frequency.recip() * 514),
                        &self.my_path,
                        |component, _| component.processor_rdy.store(true),
                    );

                    // Read off OAM data immediately, this is done for performance and should not
                    // have any side effects
                    let _ = self.cpu_address_space.read(
                        page as usize,
                        self.timestamp,
                        None,
                        &mut self.state.oam.data,
                    );
                }
                _ => {
                    unreachable!("{:?}", register);
                }
            }
        }

        Ok(())
    }

    fn access_framebuffer(&mut self, _path: &FluxEmuPath) -> &dyn Any {
        self.backend.as_mut().unwrap().access_framebuffer()
    }

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        let backend = self.backend.as_mut().unwrap();

        for now in context.allocate(self.period, None) {
            self.timestamp = now;

            if self.state.cycle_counter.y == 261 {
                if self.state.cycle_counter.x == 257 && self.state.background.rendering_enabled {
                    let t =
                        VramAddressPointerContents::from(self.state.shadow_vram_address_pointer);
                    let mut v = VramAddressPointerContents::from(self.state.vram_address_pointer);

                    v.nametable.x = t.nametable.x;
                    v.coarse.x = t.coarse.x;

                    self.state.vram_address_pointer = u16::from(v);
                }

                if let 280..=304 = self.state.cycle_counter.x
                    && self.state.background.rendering_enabled
                {
                    let t =
                        VramAddressPointerContents::from(self.state.shadow_vram_address_pointer);
                    let mut v = VramAddressPointerContents::from(self.state.vram_address_pointer);

                    v.nametable.y = t.nametable.y;
                    v.coarse.y = t.coarse.y;
                    v.fine_y = t.fine_y;

                    self.state.vram_address_pointer = u16::from(v);
                }

                if let 305..=320 = self.state.cycle_counter.x
                    && self.state.background.rendering_enabled
                {
                    self.state.drive_background_pipeline::<R>(
                        &self.ppu_address_space,
                        &mut self.ppu_address_space_cache,
                        self.timestamp,
                    );
                }
            }

            if (0..R::VISIBLE_SCANLINES).contains(&self.state.cycle_counter.y) {
                if self.state.cycle_counter.x == 1 {
                    // Technically the NES does it over 64 cycles
                    self.state.oam.secondary_data.clear();
                }

                if let 1..=256 = self.state.cycle_counter.x {
                    let scanline_position_x = self.state.cycle_counter.x - 1;

                    self.state.drive_background_pipeline::<R>(
                        &self.ppu_address_space,
                        &mut self.ppu_address_space_cache,
                        self.timestamp,
                    );

                    let mut sprite_pixel = None;

                    let potential_sprite = self
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
                        sprite_pixel = Some(self.state.calculate_sprite_color::<R>(
                            &self.ppu_address_space,
                            &mut self.ppu_address_space_cache,
                            self.timestamp,
                            sprite.oam,
                            color_index,
                        ));
                    }

                    // Extract out and combine pattern bits
                    let high = (self.state.background.pattern_high_shift
                        >> (15 - self.state.background.fine_x_scroll))
                        & 1;

                    let low = (self.state.background.pattern_low_shift
                        >> (15 - self.state.background.fine_x_scroll))
                        & 1;

                    let color_index = (high << 1) | low;

                    // Extract out attribute bits
                    let attribute = (self.state.background.attribute_shift >> 30) & 0b11;

                    // Shift pipeline forward
                    self.state.background.attribute_shift <<= 2;
                    self.state.background.pattern_low_shift <<= 1;
                    self.state.background.pattern_high_shift <<= 1;

                    let background_pixel = self.state.calculate_background_color::<R>(
                        &self.ppu_address_space,
                        &mut self.ppu_address_space_cache,
                        self.timestamp,
                        attribute as u8,
                        color_index as u8,
                    );

                    let pixel = if self.state.oam.rendering_enabled {
                        sprite_pixel
                    } else {
                        None
                    }
                    .or(if self.state.background.rendering_enabled {
                        Some(background_pixel)
                    } else {
                        None
                    })
                    .unwrap_or(BLACK);

                    backend.modify_staging_buffer(|mut staging_buffer_guard| {
                        staging_buffer_guard[(
                            scanline_position_x as usize,
                            self.state.cycle_counter.y as usize,
                        )] = pixel.into();
                    });
                }

                if let 65..=256 = self.state.cycle_counter.x {
                    let sprite_index = (self.state.cycle_counter.x - 65) / 2;
                    let oam_data_index = sprite_index * 4;

                    if sprite_index < 64 {
                        match self.state.oam.sprite_evaluation_state {
                            SpriteEvaluationState::InspectingY => {
                                let sprite_y = self.state.oam.data[oam_data_index as usize];

                                self.state.oam.sprite_evaluation_state =
                                    SpriteEvaluationState::Evaluating { sprite_y };
                            }
                            SpriteEvaluationState::Evaluating { sprite_y } => {
                                if (u16::from(sprite_y)..u16::from(sprite_y) + 8)
                                    .contains(&(self.state.cycle_counter.y))
                                {
                                    let mut bytes = [0; 4];
                                    bytes.copy_from_slice(
                                        &self.state.oam.data[RangeInclusive::from_start_and_length(
                                            oam_data_index as usize,
                                            4,
                                        )],
                                    );

                                    let sprite = OamSprite::from_bytes(bytes);

                                    if self.state.oam.secondary_data.try_push(sprite).is_err() {
                                        // TODO: Handle sprite overflow flag
                                    }
                                }

                                self.state.oam.sprite_evaluation_state =
                                    SpriteEvaluationState::InspectingY;
                            }
                        }
                    }
                }

                if self.state.cycle_counter.x == 256 && self.state.background.rendering_enabled {
                    let mut vram_address_pointer_contents =
                        VramAddressPointerContents::from(self.state.vram_address_pointer);

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

                    self.state.vram_address_pointer = u16::from(vram_address_pointer_contents);
                }

                if self.state.cycle_counter.x == 257 {
                    self.state.oam.currently_rendering_sprites.clear();

                    if self.state.background.rendering_enabled {
                        let t = VramAddressPointerContents::from(
                            self.state.shadow_vram_address_pointer,
                        );
                        let mut v =
                            VramAddressPointerContents::from(self.state.vram_address_pointer);

                        v.nametable.x = t.nametable.x;
                        v.coarse.x = t.coarse.x;

                        self.state.vram_address_pointer = u16::from(v);
                    }
                }

                if let 257..=320 = self.state.cycle_counter.x {
                    self.state.drive_sprite_pipeline::<R>(
                        &self.ppu_address_space,
                        &mut self.ppu_address_space_cache,
                        self.timestamp,
                    );
                }

                if let 321..=336 = self.state.cycle_counter.x {
                    self.state.drive_background_pipeline::<R>(
                        &self.ppu_address_space,
                        &mut self.ppu_address_space_cache,
                        self.timestamp,
                    );
                }
            }

            self.state.cycle_counter.x += 1;

            if self.state.cycle_counter.x >= TOTAL_SCANLINE_LENGTH {
                self.state.cycle_counter.x = 0;
                self.state.cycle_counter.y += 1;
            }

            if self.state.cycle_counter.y >= R::TOTAL_SCANLINES {
                self.state.cycle_counter.y = 0;
            }
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= self.period
    }
}
