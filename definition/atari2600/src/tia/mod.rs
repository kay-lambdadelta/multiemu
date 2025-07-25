use crate::tia::{
    backend::TiaDisplayBackend,
    config::TiaConfig,
    memory::{ReadRegisters, WriteRegisters},
};
pub(crate) use backend::SupportedGraphicsApiTia;
use bitvec::{array::BitArray, order::Lsb0, view::BitView};
use color::TiaColor;
use multiemu_runtime::{
    component::{Component, SnapshotError},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, ReadMemoryRecord, WriteMemoryRecord,
    },
};
use multiemu_save::ComponentVersion;
use nalgebra::{DMatrix, Point2};
use palette::Srgba;
use rangemap::RangeInclusiveMap;
use region::Region;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Mutex,
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
    state: Mutex<State>,
    backend: Mutex<G::Backend<R>>,
    config: TiaConfig<R>,
}

impl<R: Region, G: SupportedGraphicsApiTia> Component for Tia<R, G> {
    fn load_snapshot(
        &self,
        snapshot_version: ComponentVersion,
        data: &[u8],
    ) -> Result<(), SnapshotError> {
        let mut state_guard = self.state.lock().unwrap();
        let mut backend_guard = self.backend.lock().unwrap();

        match snapshot_version {
            0 => {
                let snapshot: TiaSnapshotV0 =
                    bincode::serde::decode_from_slice(data, bincode::config::standard())
                        .unwrap()
                        .0;

                *state_guard = snapshot.state;

                backend_guard.modify_staging_buffer(|mut framebuffer| {
                    framebuffer.copy_from(&snapshot.buffer);
                });
            }
            _ => return Err(SnapshotError::InvalidVersion),
        }

        Ok(())
    }

    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let data = &mut buffer[0];
        let mut state_guard = self.state.lock().unwrap();

        if let Some(address) = ReadRegisters::from_repr(address) {
            tracing::debug!("Reading from TIA register: {:?}", address);

            self.handle_read_register(data, &mut state_guard, address);

            Ok(())
        } else {
            Err(RangeInclusiveMap::from_iter([(
                address..=(address + (buffer.len() - 1)),
                ReadMemoryRecord::Denied,
            )])
            .into())
        }
    }

    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let data = buffer[0];
        let data_bits = data.view_bits::<Lsb0>();
        let mut state_guard = self.state.lock().unwrap();

        if let Some(address) = WriteRegisters::from_repr(address) {
            tracing::debug!("Writing to TIA register: {:?} = {:02x}", address, data);

            self.handle_write_register(data, data_bits, &mut state_guard, address);

            Ok(())
        } else {
            Err(RangeInclusiveMap::from_iter([(
                address..=(address + (buffer.len() - 1)),
                WriteMemoryRecord::Denied,
            )])
            .into())
        }
    }
}
