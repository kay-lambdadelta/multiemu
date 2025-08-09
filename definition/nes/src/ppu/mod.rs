use crate::INes;
use bitvec::{prelude::Lsb0, view::BitView};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, ReadMemoryRecord, WriteMemoryRecord,
    },
    platform::Platform,
};
use nalgebra::Vector2;
use serde::{Deserialize, Serialize};
use std::{ops::RangeInclusive, sync::Mutex};

// mod backend;
mod color;
mod region;
mod task;

const PPUCTRL: Address = 0x2000;
const PPUMASK: Address = 0x2001;
const PPUSTATUS: Address = 0x2002;
const OAMADDR_ADDRESS: Address = 0x2003;

pub struct OamData {}

impl OamData {
    const ADDRESS: Address = 0x2004;
}

const PPUSCROLL_ADDRESS: Address = 0x2005;
const PPUADDR_ADDRESS: Address = 0x2006;
const PPUDATA_ADDRESS: Address = 0x2007;
const OAMDMA_ADDRESS: Address = 0x4014;

pub const NAME_TABLE_ADDRESSES: [RangeInclusive<Address>; 4] = [
    0x2000..=0x23ff,
    0x2400..=0x27ff,
    0x2800..=0x2bff,
    0x2c00..=0x2fff,
];

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ColorEmphasis {
    pub red: bool,
    pub green: bool,
    pub blue: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct State {
    base_nametable_address: u16,
    sprite_8x8_pattern_table_address: u16,
    background_pattern_table_address: u16,
    sprite_size: Vector2<u16>,
    vblank_nmi: bool,
    greyscale: bool,
    show_background_leftmost_pixels: bool,
    show_sprites_leftmost_pixels: bool,
    background_rendering_enabled: bool,
    sprite_rendering_enabled: bool,
    color_emphasis: ColorEmphasis,
}

#[derive(Debug)]
pub struct NesPpuConfig<'a> {
    pub ines: &'a INes,
    pub cpu_address_space: AddressSpaceHandle,
    pub ppu_address_space: AddressSpaceHandle,
}

#[derive(Debug)]
pub struct NesPpu {
    state: Mutex<State>,
}

impl Component for NesPpu {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        match address {
            PPUSTATUS => {}
            _ => {
                unreachable!()
            }
        }

        Ok(())
    }

    fn write_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let buffer_bits = buffer.view_bits::<Lsb0>();

        match address {
            PPUCTRL => {}
            PPUMASK => {}
            _ => {
                unreachable!()
            }
        }

        Ok(())
    }
}

impl<'a, P: Platform> ComponentConfig<P> for NesPpuConfig<'a> {
    type Component = NesPpu;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        component_builder.build_local(NesPpu {
            state: Mutex::new(State {
                base_nametable_address: 0x2000,
                sprite_8x8_pattern_table_address: 0,
                background_pattern_table_address: 0,
                sprite_size: Vector2::new(8, 8),
                vblank_nmi: false,
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
            }),
        });

        Ok(())
    }
}
