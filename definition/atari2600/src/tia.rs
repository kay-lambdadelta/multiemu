use bitvec::{
    prelude::{Lsb0, Msb0},
    view::BitView,
};
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceId,
        callbacks::Memory,
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use nalgebra::Point2;
use petgraph::prelude::UnGraphMap;
use rangemap::RangeInclusiveMap;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use strum::{EnumIter, FromRepr, IntoEnumIterator};

use crate::CPU_ADDRESS_SPACE;

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
enum ReadRegisters {
    Cxm0p = 0x000,
    Cxm1p = 0x001,
    Cxp0fb = 0x002,
    Cxp1fb = 0x003,
    Cxm0fb = 0x004,
    Cxm1fb = 0x005,
    Cxblpf = 0x006,
    Cxppmm = 0x007,
    Inpt0 = 0x008,
    Inpt1 = 0x009,
    Inpt2 = 0x00a,
    Inpt3 = 0x00b,
    Inpt4 = 0x00c,
    Inpt5 = 0x00d,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
enum WriteRegisters {
    Vsync = 0x000,
    Vblank = 0x001,
    Wsync = 0x002,
    Rsync = 0x003,
    Nusiz0 = 0x004,
    Nusiz1 = 0x005,
    Colup0 = 0x006,
    Colup1 = 0x007,
    Colupf = 0x008,
    Colubk = 0x009,
    Ctrlpf = 0x00a,
    Refp0 = 0x00b,
    Refp1 = 0x00c,
    Pf0 = 0x00d,
    Pf1 = 0x00e,
    Pf2 = 0x00f,
    Resp0 = 0x010,
    Resp1 = 0x011,
    Resm0 = 0x012,
    Resm1 = 0x013,
    Resbl = 0x014,
    Audc0 = 0x015,
    Audc1 = 0x016,
    Audf0 = 0x017,
    Audf1 = 0x018,
    Audv0 = 0x019,
    Audv1 = 0x01a,
    Grp0 = 0x01b,
    Grp1 = 0x01c,
    Enam0 = 0x01d,
    Enam1 = 0x01e,
    Enabl = 0x01f,
    Hmp0 = 0x020,
    Hmp1 = 0x021,
    Hmm0 = 0x022,
    Hmm1 = 0x023,
    Hmbl = 0x024,
    Vdelp0 = 0x025,
    Vdelp1 = 0x026,
    Vdelpl = 0x027,
    Resmp0 = 0x028,
    Resmp1 = 0x029,
    Hmove = 0x02a,
    Hmclr = 0x02b,
    Cxclr = 0x02c,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, FromRepr, EnumIter)]
enum ObjectId {
    Player0,
    Player1,
    Missile0,
    Missile1,
    Playfield,
    Ball,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Hash, Clone, Copy)]
enum ObjectPosition {
    LockedToPlayer,
    Position(Point2<u8>),
}

impl Default for ObjectPosition {
    fn default() -> Self {
        Self::Position(Point2::new(0, 0))
    }
}

#[derive(Debug)]
struct State {
    objects: HashMap<ObjectId, Object>,
    collision_matrix: UnGraphMap<ObjectId, ()>,
    current_scanline: u8,
}

impl Default for State {
    fn default() -> Self {
        Self {
            collision_matrix: UnGraphMap::default(),
            objects: ObjectId::iter().map(|id| (id, Object::default())).collect(),
            current_scanline: 0,
        }
    }
}

#[derive(Default, Debug)]
struct Object {
    position: ObjectPosition,
}

pub(crate) struct Tia {
    state: Arc<Mutex<State>>,
}

impl Component for Tia {}

impl FromConfig for Tia {
    type Config = ();
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        quirks: Self::Quirks,
    ) {
        let state = Arc::new(Mutex::new(State::default()));

        component_builder
            .insert_memory(
                [(0x000..=0x03d, CPU_ADDRESS_SPACE)],
                MemoryCallback {
                    state: state.clone(),
                },
            )
            .build(Tia { state });
    }
}

#[derive(Debug)]
struct MemoryCallback {
    state: Arc<Mutex<State>>,
}

impl Memory for MemoryCallback {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceId,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        let data = buffer[0].view_bits::<Lsb0>();
        let mut state_guard = self.state.lock().unwrap();

        if let Some(address) = WriteRegisters::from_repr(address) {
            match address {
                WriteRegisters::Vsync => todo!(),
                WriteRegisters::Vblank => todo!(),
                WriteRegisters::Wsync => todo!(),
                WriteRegisters::Rsync => todo!(),
                WriteRegisters::Nusiz0 => todo!(),
                WriteRegisters::Nusiz1 => todo!(),
                WriteRegisters::Colup0 => todo!(),
                WriteRegisters::Colup1 => todo!(),
                WriteRegisters::Colupf => todo!(),
                WriteRegisters::Colubk => todo!(),
                WriteRegisters::Ctrlpf => todo!(),
                WriteRegisters::Refp0 => todo!(),
                WriteRegisters::Refp1 => todo!(),
                WriteRegisters::Pf0 => todo!(),
                WriteRegisters::Pf1 => todo!(),
                WriteRegisters::Pf2 => todo!(),
                WriteRegisters::Resp0 => todo!(),
                WriteRegisters::Resp1 => todo!(),
                WriteRegisters::Resm0 => todo!(),
                WriteRegisters::Resm1 => todo!(),
                WriteRegisters::Resbl => todo!(),
                WriteRegisters::Audc0 => todo!(),
                WriteRegisters::Audc1 => todo!(),
                WriteRegisters::Audf0 => todo!(),
                WriteRegisters::Audf1 => todo!(),
                WriteRegisters::Audv0 => todo!(),
                WriteRegisters::Audv1 => todo!(),
                WriteRegisters::Grp0 => todo!(),
                WriteRegisters::Grp1 => todo!(),
                WriteRegisters::Enam0 => todo!(),
                WriteRegisters::Enam1 => todo!(),
                WriteRegisters::Enabl => todo!(),
                WriteRegisters::Hmp0 => todo!(),
                WriteRegisters::Hmp1 => todo!(),
                WriteRegisters::Hmm0 => todo!(),
                WriteRegisters::Hmm1 => todo!(),
                WriteRegisters::Hmbl => todo!(),
                WriteRegisters::Vdelp0 => todo!(),
                WriteRegisters::Vdelp1 => todo!(),
                WriteRegisters::Vdelpl => todo!(),
                WriteRegisters::Resmp0 => {
                    if data[1] {
                        state_guard
                            .objects
                            .get_mut(&ObjectId::Missile0)
                            .unwrap()
                            .position = ObjectPosition::LockedToPlayer;
                    }
                }
                WriteRegisters::Resmp1 => {
                    if data[1] {
                        state_guard
                            .objects
                            .get_mut(&ObjectId::Missile1)
                            .unwrap()
                            .position = ObjectPosition::LockedToPlayer;
                    }
                }
                WriteRegisters::Hmove => todo!(),
                WriteRegisters::Hmclr => todo!(),
                WriteRegisters::Cxclr => {
                    // Clear collision latches

                    state_guard.collision_matrix.clear();
                }
            }
        } else {
            errors.insert(
                address..=(address + (buffer.len() - 1)),
                WriteMemoryRecord::Denied,
            );
        }
    }

    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        let state_guard = self.state.lock().unwrap();

        // Adjust for the mirror
        let actual_address = if (0x30..=0x3d).contains(&address) {
            address - 0x30
        } else {
            address
        };

        if let Some(register) = ReadRegisters::from_repr(actual_address) {
            match register {
                ReadRegisters::Cxm0p => {
                    self.read_collision_register(
                        buffer,
                        [ObjectId::Player0, ObjectId::Missile0],
                        [ObjectId::Player1, ObjectId::Missile0],
                        &state_guard.collision_matrix,
                    );
                }
                ReadRegisters::Cxm1p => {
                    self.read_collision_register(
                        buffer,
                        [ObjectId::Player0, ObjectId::Missile1],
                        [ObjectId::Player1, ObjectId::Missile1],
                        &state_guard.collision_matrix,
                    );
                }
                ReadRegisters::Cxp0fb => {
                    self.read_collision_register(
                        buffer,
                        [ObjectId::Player0, ObjectId::Playfield],
                        [ObjectId::Player0, ObjectId::Ball],
                        &state_guard.collision_matrix,
                    );
                }
                ReadRegisters::Cxp1fb => {
                    self.read_collision_register(
                        buffer,
                        [ObjectId::Player1, ObjectId::Playfield],
                        [ObjectId::Player1, ObjectId::Ball],
                        &state_guard.collision_matrix,
                    );
                }
                ReadRegisters::Cxm0fb => {
                    self.read_collision_register(
                        buffer,
                        [ObjectId::Missile0, ObjectId::Playfield],
                        [ObjectId::Missile0, ObjectId::Ball],
                        &state_guard.collision_matrix,
                    );
                }
                ReadRegisters::Cxm1fb => {
                    self.read_collision_register(
                        buffer,
                        [ObjectId::Missile1, ObjectId::Playfield],
                        [ObjectId::Missile1, ObjectId::Ball],
                        &state_guard.collision_matrix,
                    );
                }
                ReadRegisters::Cxblpf => {
                    let collision = state_guard
                        .collision_matrix
                        .contains_edge(ObjectId::Ball, ObjectId::Playfield);
                    let buffer_bits = buffer.view_bits_mut::<Msb0>();

                    buffer_bits.set(0, collision);
                }
                ReadRegisters::Cxppmm => {
                    self.read_collision_register(
                        buffer,
                        [ObjectId::Player0, ObjectId::Player1],
                        [ObjectId::Missile0, ObjectId::Missile1],
                        &state_guard.collision_matrix,
                    );
                }
                ReadRegisters::Inpt0 => todo!(),
                ReadRegisters::Inpt1 => todo!(),
                ReadRegisters::Inpt2 => todo!(),
                ReadRegisters::Inpt3 => todo!(),
                ReadRegisters::Inpt4 => todo!(),
                ReadRegisters::Inpt5 => todo!(),
            }
        } else {
            errors.insert(
                address..=(address + (buffer.len() - 1)),
                ReadMemoryRecord::Denied,
            );
        }
    }
}

impl MemoryCallback {
    #[inline]
    fn read_collision_register(
        &self,
        buffer: &mut [u8],
        pair1: [ObjectId; 2],
        pair2: [ObjectId; 2],
        collision_matrix: &UnGraphMap<ObjectId, ()>,
    ) {
        let collision1 = collision_matrix.contains_edge(pair1[0], pair1[1]);
        let collision2 = collision_matrix.contains_edge(pair2[0], pair2[1]);

        let buffer_bits = buffer.view_bits_mut::<Msb0>();

        buffer_bits.set(0, collision1);
        buffer_bits.set(1, collision2);
    }
}
