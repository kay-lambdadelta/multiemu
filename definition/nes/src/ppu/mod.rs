use crate::{
    INes,
    ppu::{
        backend::{PpuDisplayBackend, SupportedGraphicsApiPpu},
        background::{BackgroundPipelineState, BackgroundState},
        oam::{OamDmaTask, OamState, SpriteEvaluationState},
        region::Region,
        state::State,
        task::Driver,
    },
};
use arrayvec::ArrayVec;
use bitvec::{field::BitField, prelude::Lsb0, view::BitView};
use multiemu_definition_mos6502::{Mos6502, RdyFlag};
use multiemu_range::ContiguousRange;
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentPath, ResourcePath},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, MemoryAccessTable, ReadMemoryError, WriteMemoryError},
    platform::Platform,
};
use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    marker::PhantomData,
    ops::RangeInclusive,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use strum::FromRepr;

pub mod backend;
mod background;
mod color;
mod oam;
pub mod region;
mod state;
mod task;

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
pub const NAMETABLE_SIZE: Address = 0x400;

pub const BACKGROUND_PALETTE_BASE_ADDRESS: Address = 0x3f00;

const DUMMY_SCANLINE_COUNT: u16 = 1;
const VISIBLE_SCANLINE_LENGTH: u16 = 256;
const HBLANK_LENGTH: u16 = 85;
const TOTAL_SCANLINE_LENGTH: u16 = VISIBLE_SCANLINE_LENGTH + HBLANK_LENGTH;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ColorEmphasis {
    /// This actually means green on pal/dendy
    pub red: bool,
    /// This actually means red on pal/dendy
    pub green: bool,
    pub blue: bool,
}

#[derive(Debug)]
pub struct NesPpuConfig<'a, R: Region> {
    pub ines: &'a INes,
    pub cpu_address_space: AddressSpaceId,
    pub ppu_address_space: AddressSpaceId,
    pub processor: ComponentPath,
    pub _phantom: PhantomData<R>,
}

#[derive(Debug)]
pub struct Ppu<R: Region, G: SupportedGraphicsApiPpu> {
    state: State,
    backend: Option<G::Backend<R>>,
    cpu_address_space: AddressSpaceId,
    ppu_address_space: AddressSpaceId,
    memory_access_table: Arc<MemoryAccessTable>,
    processor_rdy: Arc<RdyFlag>,
}

impl<R: Region, G: SupportedGraphicsApiPpu> Component for Ppu<R, G> {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        for (address, buffer) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter_mut())
        {
            let register = CpuAccessibleRegister::from_repr(address as u16).unwrap();
            tracing::trace!("Reading from PPU register: {:?}", register);

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
                    *buffer = self.memory_access_table.read_le_value(
                        self.state.ppu_addr as usize,
                        self.ppu_address_space,
                        avoid_side_effects,
                    )?;
                }
                _ => {
                    unreachable!("{:?}", register);
                }
            }
        }

        Ok(())
    }

    fn write_memory(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        for (address, buffer) in
            RangeInclusive::from_start_and_length(address, buffer.len()).zip(buffer.iter())
        {
            let register = CpuAccessibleRegister::from_repr(address as u16).unwrap();
            tracing::trace!("Writing to PPU register: {:?}", register);

            match register {
                CpuAccessibleRegister::PpuCtrl => {
                    let data_bits = buffer.view_bits::<Lsb0>();

                    self.state.nametable_base = NAMETABLE_BASE_ADDRESS as u16
                        + (data_bits[0..=1].load::<u16>() * NAMETABLE_SIZE as u16);

                    self.state.ppu_addr_increment_amount = if data_bits[2] { 32 } else { 1 };

                    self.state.oam.sprite_8x8_pattern_table_address =
                        if data_bits[3] { 0x1000 } else { 0x0000 };
                    self.state.background.pattern_table_base =
                        if data_bits[4] { 0x1000 } else { 0x0000 };

                    self.state.vblank_nmi_enabled = data_bits[7];
                }
                CpuAccessibleRegister::PpuMask => {
                    let data_bits = buffer.view_bits::<Lsb0>();

                    self.state.greyscale = data_bits[0];

                    self.state.show_background_leftmost_pixels = data_bits[1];
                    self.state.oam.show_sprites_leftmost_pixels = data_bits[2];

                    self.state.background.rendering_enabled = data_bits[3];
                    self.state.oam.sprite_rendering_enabled = data_bits[4];

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
                    let bits = buffer.view_bits::<Lsb0>();

                    let fine_scroll = bits[0..=2].load::<u8>();
                    let coarse_scroll = bits[3..=7].load::<u8>();

                    if !self.state.ppu_addr_ppu_scroll_write_phase {
                        self.state.background.fine_scroll.x = fine_scroll;
                        self.state.background.coarse_scroll.x = coarse_scroll;
                    } else {
                        self.state.background.fine_scroll.y = fine_scroll;
                        self.state.background.coarse_scroll.y = coarse_scroll;
                    }

                    self.state.ppu_addr_ppu_scroll_write_phase =
                        !self.state.ppu_addr_ppu_scroll_write_phase;
                }
                CpuAccessibleRegister::PpuAddr => {
                    let mut unpacked_address = self.state.ppu_addr.to_be_bytes();
                    unpacked_address[self.state.ppu_addr_ppu_scroll_write_phase as usize] = *buffer;
                    self.state.ppu_addr_ppu_scroll_write_phase =
                        !self.state.ppu_addr_ppu_scroll_write_phase;
                    self.state.ppu_addr = u16::from_be_bytes(unpacked_address);
                }
                CpuAccessibleRegister::PpuData => {
                    tracing::debug!(
                        "CPU is sending data to 0x{:04x} in the PPU address space: {:02x}",
                        self.state.ppu_addr,
                        buffer
                    );

                    self.memory_access_table.write_le_value(
                        self.state.ppu_addr as usize,
                        self.ppu_address_space,
                        *buffer,
                    )?;

                    // Redirect into the ppu address space
                    self.state.ppu_addr = self
                        .state
                        .ppu_addr
                        .wrapping_add(self.state.ppu_addr_increment_amount as u16);
                }
                CpuAccessibleRegister::OamDma => {
                    let page = (*buffer as u16) << 8;
                    // Halt the processor
                    self.processor_rdy.store(false);
                    self.state.oam.cpu_dma_countdown = 514;

                    // Read off OAM data immediately, this is done for performance and should not have any side effects
                    let _ = self.memory_access_table.read(
                        page as usize,
                        self.cpu_address_space,
                        false,
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

    fn access_framebuffer<'a>(
        &'a mut self,
        _display_path: &ResourcePath,
        callback: Box<dyn FnOnce(&dyn Any) + 'a>,
    ) {
        self.backend
            .as_mut()
            .unwrap()
            .access_framebuffer(|framebuffer| callback(framebuffer));
    }
}

impl<'a, R: Region, P: Platform<GraphicsApi: SupportedGraphicsApiPpu>> ComponentConfig<P>
    for NesPpuConfig<'a, R>
{
    type Component = Ppu<R, P::GraphicsApi>;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let access_table = component_builder.memory_access_table();

        let (component_builder, _) = component_builder.insert_display("tv");

        let processor_nmi = component_builder
            .registry()
            .interact::<Mos6502, _>(&self.processor, |component| component.nmi())
            .unwrap();

        let processor_rdy = component_builder
            .registry()
            .interact::<Mos6502, _>(&self.processor, |component| component.rdy())
            .unwrap();

        component_builder
            .insert_task_mut(
                "driver",
                R::master_clock() / 4,
                Driver {
                    processor_nmi,
                    ppu_address_space: self.ppu_address_space,
                },
            )
            // At the same speed as the processor
            .insert_task_mut("oam_dma_counter", R::master_clock() / 3, OamDmaTask)
            .set_lazy_component_initializer(|component, data| {
                component.backend = Some(PpuDisplayBackend::new(
                    data.component_graphics_initialization_data.clone(),
                ));
            })
            .memory_map_write(
                CpuAccessibleRegister::PpuCtrl as usize..=CpuAccessibleRegister::PpuCtrl as usize,
                self.cpu_address_space,
            )
            .memory_map_write(
                CpuAccessibleRegister::PpuScroll as usize
                    ..=CpuAccessibleRegister::PpuScroll as usize,
                self.cpu_address_space,
            )
            .memory_map_write(
                CpuAccessibleRegister::PpuMask as usize..=CpuAccessibleRegister::PpuMask as usize,
                self.cpu_address_space,
            )
            .memory_map_read(
                CpuAccessibleRegister::PpuStatus as usize
                    ..=CpuAccessibleRegister::PpuStatus as usize,
                self.cpu_address_space,
            )
            .memory_map(
                CpuAccessibleRegister::PpuAddr as usize..=CpuAccessibleRegister::PpuAddr as usize,
                self.cpu_address_space,
            )
            .memory_map(
                CpuAccessibleRegister::PpuData as usize..=CpuAccessibleRegister::PpuData as usize,
                self.cpu_address_space,
            )
            .memory_map(
                CpuAccessibleRegister::OamAddr as usize..=CpuAccessibleRegister::OamAddr as usize,
                self.cpu_address_space,
            )
            .memory_map(
                CpuAccessibleRegister::OamData as usize..=CpuAccessibleRegister::OamData as usize,
                self.cpu_address_space,
            )
            .memory_map_write(
                CpuAccessibleRegister::OamDma as usize..=CpuAccessibleRegister::OamDma as usize,
                self.cpu_address_space,
            );

        Ok(Ppu {
            state: State {
                nametable_base: 0x2000,
                sprite_size: Vector2::new(8, 8),
                vblank_nmi_enabled: false,
                reset_cpu_nmi: false,
                entered_vblank: AtomicBool::new(false),
                greyscale: false,
                show_background_leftmost_pixels: false,
                color_emphasis: ColorEmphasis {
                    red: false,
                    green: false,
                    blue: false,
                },
                // Start it on the dummy scanline
                cycle_counter: Point2::new(0, 261),
                background_pipeline_state: BackgroundPipelineState::FetchingNametable,
                awaiting_memory_access: true,
                ppu_addr: 0,
                ppu_addr_ppu_scroll_write_phase: false,
                ppu_addr_increment_amount: 1,
                background: BackgroundState {
                    fine_scroll: Vector2::default(),
                    coarse_scroll: Vector2::default(),
                    pattern_table_base: 0x0000,
                    rendering_enabled: true,
                    pattern_low_shift: 0,
                    pattern_high_shift: 0,
                    attribute_shift: 0,
                },
                oam: OamState {
                    data: rand::random(),
                    oam_addr: 0x00,
                    sprite_evaluation_state: SpriteEvaluationState::InspectingY,
                    queued_sprites: ArrayVec::new(),
                    show_sprites_leftmost_pixels: true,
                    sprite_8x8_pattern_table_address: 0x0000,
                    sprite_rendering_enabled: true,
                    cpu_dma_countdown: 0,
                },
            },
            backend: None,
            ppu_address_space: self.ppu_address_space,
            cpu_address_space: self.cpu_address_space,
            memory_access_table: access_table,
            processor_rdy,
        })
    }
}
