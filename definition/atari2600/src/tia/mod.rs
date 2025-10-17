use crate::tia::{
    backend::TiaDisplayBackend,
    memory::{ReadRegisters, WriteRegisters},
};
pub(crate) use backend::SupportedGraphicsApiTia;
use bitvec::{array::BitArray, order::Lsb0, view::BitView};
use color::TiaColor;
use multiemu_definition_mos6502::RdyFlag;
use multiemu_runtime::{
    component::{Component, ResourcePath},
    memory::{
        Address, AddressSpaceId, ReadMemoryError, ReadMemoryErrorType, WriteMemoryError,
        WriteMemoryErrorType,
    },
};
use nalgebra::{DMatrix, Point2};
use palette::Srgba;
use region::Region;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

mod backend;
mod color;
pub(crate) mod config;
mod memory;
pub mod region;
mod task;

const HBLANK_LENGTH: u16 = 68;
const VISIBLE_SCANLINE_LENGTH: u16 = 160;
const SCANLINE_LENGTH: u16 = HBLANK_LENGTH + VISIBLE_SCANLINE_LENGTH;

#[derive(Debug, Serialize, Deserialize)]
struct TiaSnapshotV0 {
    state: State,
    buffer: DMatrix<Srgba<u8>>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
enum ObjectId {
    Player0,
    Player1,
    Missile0,
    Missile1,
    Playfield,
    Ball,
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
enum InputControl {
    #[default]
    Normal,
    LatchedOrDumped,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct State {
    collision_matrix: HashMap<ObjectId, HashSet<ObjectId>>,
    vblank_active: bool,
    cycles_waiting_for_vsync: Option<u16>,
    reset_rdy_on_scanline_end: bool,
    input_control: [InputControl; 6],
    electron_beam: Point2<u16>,
    missiles: [Missile; 2],
    ball: Ball,
    players: [Player; 2],
    playfield: Playfield,
    high_playfield_ball_priority: bool,
    background_color: TiaColor,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Missile {
    position: u16,
    enabled: bool,
    motion: i8,
    color: TiaColor,
    /// Locked to player and invisible
    locked: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum DelayEnableChangeBall {
    #[default]
    Disabled,
    Enabled(Option<bool>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Ball {
    position: u16,
    enabled: bool,
    delay_enable_change: DelayEnableChangeBall,
    motion: i8,
    color: TiaColor,
    size: u8,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Playfield {
    mirror: bool,
    color: TiaColor,
    score_mode: bool,
    // 20 bits
    data: BitArray<[u8; 3], Lsb0>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum DelayChangeGraphicPlayer {
    #[default]
    Disabled,
    Enabled(Option<u8>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Player {
    position: u16,
    graphic: u8,
    mirror: bool,
    delay_change_graphic: DelayChangeGraphicPlayer,
    motion: i8,
    color: TiaColor,
}

#[derive(Debug)]
pub(crate) struct Tia<R: Region, G: SupportedGraphicsApiTia> {
    state: State,
    backend: Option<G::Backend<R>>,
    cpu_rdy: Arc<RdyFlag>,
}

impl<R: Region, G: SupportedGraphicsApiTia> Component for Tia<R, G> {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        _avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        let data = &mut buffer[0];

        if let Some(address) = ReadRegisters::from_repr(address) {
            tracing::debug!("Reading from TIA register: {:?}", address);

            self.handle_read_register(data, address);

            Ok(())
        } else {
            Err(ReadMemoryError(
                std::iter::once((
                    address..=(address + (buffer.len() - 1)),
                    ReadMemoryErrorType::Denied,
                ))
                .collect(),
            ))
        }
    }

    fn write_memory(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        let data = buffer[0];
        let data_bits = data.view_bits::<Lsb0>();

        if let Some(address) = WriteRegisters::from_repr(address) {
            tracing::debug!("Writing to TIA register: {:?} = {:02x}", address, data);

            self.handle_write_register(data, data_bits, address);

            Ok(())
        } else {
            Err(WriteMemoryError(
                std::iter::once((
                    address..=(address + (buffer.len() - 1)),
                    WriteMemoryErrorType::Denied,
                ))
                .collect(),
            ))
        }
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
