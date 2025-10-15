use crate::{
    INes,
    ppu::{
        backend::{PpuDisplayBackend, SupportedGraphicsApiPpu},
        region::Region,
        task::Driver,
    },
};
use bitvec::{field::BitField, prelude::Lsb0, view::BitView};
use multiemu_base::{
    component::{Component, ComponentConfig, ComponentPath, ResourcePath},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, MemoryAccessTable, ReadMemoryError, WriteMemoryError},
    platform::Platform,
};
use multiemu_definition_mos6502::Mos6502;
use nalgebra::{Point2, Vector2};
use palette::{Srgba, named::BLACK};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::VecDeque,
    marker::PhantomData,
    ops::{Not, RangeInclusive},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
use strum::FromRepr;

pub mod backend;
mod color;
pub mod region;
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

#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DrawingState {
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

#[derive(Serialize, Deserialize, Debug)]
struct State {
    nametable_base: u16,
    sprite_8x8_pattern_table_address: u16,
    background_pattern_table_base: u16,
    sprite_size: Vector2<u16>,
    vblank_nmi_enabled: bool,
    reset_cpu_nmi: bool,
    greyscale: bool,
    entered_vblank: AtomicBool,
    show_background_leftmost_pixels: bool,
    show_sprites_leftmost_pixels: bool,
    background_rendering_enabled: bool,
    sprite_rendering_enabled: bool,
    ppuaddr: u16,
    // NES documents tend to call this w
    ppuaddr_ppuscroll_write_phase: bool,
    ppuaddr_increment_amount: u8,
    color_emphasis: ColorEmphasis,
    cycle_counter: Point2<u16>,
    fine_scroll: Vector2<u8>,
    coarse_scroll: Vector2<u8>,
    awaiting_memory_access: bool,
    drawing_state: DrawingState,
    pixel_queue: VecDeque<Srgba<u8>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            nametable_base: 0x2000,
            sprite_8x8_pattern_table_address: 0,
            background_pattern_table_base: 0,
            sprite_size: Vector2::new(8, 8),
            vblank_nmi_enabled: false,
            reset_cpu_nmi: false,
            entered_vblank: AtomicBool::new(false),
            greyscale: false,
            show_background_leftmost_pixels: false,
            show_sprites_leftmost_pixels: false,
            background_rendering_enabled: false,
            sprite_rendering_enabled: false,
            color_emphasis: ColorEmphasis {
                red: false,
                green: false,
                blue: false,
            },
            // Start it on the dummy scanline
            cycle_counter: Point2::new(0, 261),
            drawing_state: DrawingState::FetchingNametable,
            awaiting_memory_access: true,
            ppuaddr: 0,
            ppuaddr_ppuscroll_write_phase: false,
            ppuaddr_increment_amount: 1,
            fine_scroll: Vector2::default(),
            coarse_scroll: Vector2::default(),
            pixel_queue: VecDeque::from_iter([BLACK.into(); 8]),
        }
    }
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
    backend: OnceLock<G::Backend<R>>,
    ppu_address_space: AddressSpaceId,
    access_table: Arc<MemoryAccessTable>,
}

impl<R: Region, G: SupportedGraphicsApiPpu> Component for Ppu<R, G> {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        let register = CpuAccessibleRegister::from_repr(address as u16).unwrap();

        tracing::trace!("Reading from PPU register: {:?}", register);

        match register {
            CpuAccessibleRegister::PpuMask => todo!(),
            CpuAccessibleRegister::PpuStatus => {
                let buffer_bits = buffer.view_bits_mut::<Lsb0>();

                // Currently in vblank
                buffer_bits.set(7, self.state.entered_vblank.swap(false, Ordering::AcqRel));
            }
            CpuAccessibleRegister::OamAddr => todo!(),
            CpuAccessibleRegister::OamData => todo!(),
            CpuAccessibleRegister::PpuScroll => todo!(),
            CpuAccessibleRegister::PpuAddr => todo!(),
            CpuAccessibleRegister::PpuData => {
                self.access_table.read(
                    self.state.ppuaddr as usize,
                    self.ppu_address_space,
                    buffer,
                )?;
            }
            CpuAccessibleRegister::OamDma => todo!(),
            _ => {
                unreachable!("{:?}", register);
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
        let data = buffer[0];

        let register = CpuAccessibleRegister::from_repr(address as u16).unwrap();

        tracing::trace!("Writing to PPU register: {:?}", register);

        match register {
            CpuAccessibleRegister::PpuCtrl => {
                let data_bits = data.view_bits::<Lsb0>();

                self.state.nametable_base = NAMETABLE_BASE_ADDRESS as u16
                    + (data_bits[0..=1].load::<u16>() * NAMETABLE_SIZE as u16);

                self.state.ppuaddr_increment_amount = if data_bits[2] { 32 } else { 1 };

                self.state.sprite_8x8_pattern_table_address =
                    if data_bits[3] { 0x1000 } else { 0x0000 };
                self.state.background_pattern_table_base =
                    if data_bits[4] { 0x1000 } else { 0x0000 };

                self.state.vblank_nmi_enabled = data_bits[7];
            }
            CpuAccessibleRegister::PpuMask => {
                let data_bits = data.view_bits::<Lsb0>();

                self.state.greyscale = data_bits[0];

                self.state.show_background_leftmost_pixels = data_bits[1];
                self.state.show_sprites_leftmost_pixels = data_bits[2];

                self.state.background_rendering_enabled = data_bits[3];
                self.state.sprite_rendering_enabled = data_bits[4];

                self.state.color_emphasis.red = data_bits[5];
                self.state.color_emphasis.green = data_bits[6];
                self.state.color_emphasis.blue = data_bits[7];
            }
            CpuAccessibleRegister::PpuStatus => todo!(),
            CpuAccessibleRegister::OamAddr => todo!(),
            CpuAccessibleRegister::OamData => todo!(),
            CpuAccessibleRegister::PpuScroll => {
                // Convert the byte into a bit slice
                let bits = data.view_bits::<Lsb0>();

                let fine_scroll = bits[0..=2].load::<u8>();
                let coarse_scroll = bits[3..=7].load::<u8>();

                if !self.state.ppuaddr_ppuscroll_write_phase {
                    self.state.fine_scroll.x = fine_scroll;
                    self.state.coarse_scroll.x = coarse_scroll;
                } else {
                    self.state.fine_scroll.y = fine_scroll;
                    self.state.coarse_scroll.y = coarse_scroll;
                }

                self.state.ppuaddr_ppuscroll_write_phase =
                    !self.state.ppuaddr_ppuscroll_write_phase;
            }
            CpuAccessibleRegister::PpuAddr => {
                let mut unpacked_address = self.state.ppuaddr.to_be_bytes();
                unpacked_address[self.state.ppuaddr_ppuscroll_write_phase as usize] = data;
                self.state.ppuaddr_ppuscroll_write_phase =
                    !self.state.ppuaddr_ppuscroll_write_phase;
                self.state.ppuaddr = u16::from_be_bytes(unpacked_address);
            }
            CpuAccessibleRegister::PpuData => {
                tracing::debug!(
                    "CPU is sending data to 0x{:04x} in the PPU address space: {:02x}",
                    self.state.ppuaddr,
                    data
                );

                self.access_table.write(
                    self.state.ppuaddr as usize,
                    self.ppu_address_space,
                    buffer,
                )?;

                // Redirect into the ppu address space
                self.state.ppuaddr = self
                    .state
                    .ppuaddr
                    .wrapping_add(self.state.ppuaddr_increment_amount as u16);
            }
            CpuAccessibleRegister::OamDma => todo!(),
        }

        Ok(())
    }

    fn access_framebuffer<'a>(
        &'a mut self,
        _display_path: &ResourcePath,
        callback: Box<dyn FnOnce(&dyn Any) + 'a>,
    ) {
        self.backend
            .get_mut()
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

        component_builder
            .insert_task_mut(
                "driver",
                R::master_clock() / 4,
                Driver {
                    processor_nmi,
                    ppu_address_space: self.ppu_address_space,
                },
            )
            .set_lazy_component_initializer(move |component, data| {
                component
                    .backend
                    .set(PpuDisplayBackend::new(
                        data.component_graphics_initialization_data.clone(),
                    ))
                    .unwrap();
            })
            .memory_map_write(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuCtrl as usize..=CpuAccessibleRegister::PpuCtrl as usize,
            )
            .memory_map_write(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuScroll as usize
                    ..=CpuAccessibleRegister::PpuScroll as usize,
            )
            .memory_map_write(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuMask as usize..=CpuAccessibleRegister::PpuMask as usize,
            )
            .memory_map_read(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuStatus as usize
                    ..=CpuAccessibleRegister::PpuStatus as usize,
            )
            .memory_map(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuAddr as usize..=CpuAccessibleRegister::PpuAddr as usize,
            )
            .memory_map(
                self.cpu_address_space,
                CpuAccessibleRegister::PpuData as usize..=CpuAccessibleRegister::PpuData as usize,
            );

        Ok(Ppu {
            state: Default::default(),
            backend: OnceLock::default(),
            ppu_address_space: self.ppu_address_space,
            access_table,
        })
    }
}
