use super::State;
use crate::tia::{
    DelayChangeGraphicPlayer, DelayEnableChangeBall, InputControl, ObjectId, ObjectPosition,
};
use bitvec::{
    field::BitField,
    order::{Lsb0, Msb0},
    view::BitView,
};
use multiemu_definition_mos6502::Mos6502;
use multiemu_machine::{
    component::component_ref::ComponentRef,
    memory::{
        AddressSpaceHandle,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use rangemap::RangeInclusiveMap;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use strum::FromRepr;

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
    Vdelbl = 0x027,
    Resmp0 = 0x028,
    Resmp1 = 0x029,
    Hmove = 0x02a,
    Hmclr = 0x02b,
    Cxclr = 0x02c,
}

#[derive(Debug)]
pub struct MemoryCallback {
    pub state: Arc<Mutex<State>>,
    pub processor: ComponentRef<Mos6502>,
}

impl WriteMemory for MemoryCallback {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        let data_bits = buffer[0].view_bits::<Lsb0>();
        let mut state_guard = self.state.lock().unwrap();

        if let Some(address) = WriteRegisters::from_repr(address) {
            tracing::debug!("Writing to TIA register: {:?} = {:02x}", address, buffer[0]);

            match address {
                WriteRegisters::Vsync => {
                    state_guard.in_vsync = data_bits[1];
                }
                WriteRegisters::Vblank => {
                    state_guard.in_vblank = data_bits[1];

                    state_guard.input_control[0] = if data_bits[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[1] = if data_bits[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[2] = if data_bits[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[3] = if data_bits[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[4] = if data_bits[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[4] = if data_bits[6] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[5] = if data_bits[6] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };
                }
                WriteRegisters::Wsync => {
                    self.processor
                        .interact(|processor| processor.set_rdy(false))
                        .unwrap();
                    state_guard.reset_rdy_on_scanline_end = true;
                }
                WriteRegisters::Rsync => {
                    state_guard.horizontal_timer = 0;
                }
                WriteRegisters::Nusiz0 => {}
                WriteRegisters::Nusiz1 => {}
                WriteRegisters::Colup0 => {}
                WriteRegisters::Colup1 => {}
                WriteRegisters::Colupf => {}
                WriteRegisters::Colubk => {}
                WriteRegisters::Ctrlpf => {}
                WriteRegisters::Refp0 => {
                    state_guard.players[0].mirror = data_bits[3];
                }
                WriteRegisters::Refp1 => {
                    state_guard.players[1].mirror = data_bits[3];
                }
                WriteRegisters::Pf0 => {}
                WriteRegisters::Pf1 => {}
                WriteRegisters::Pf2 => {}
                WriteRegisters::Resp0 => {}
                WriteRegisters::Resp1 => {}
                WriteRegisters::Resm0 => {}
                WriteRegisters::Resm1 => {}
                WriteRegisters::Resbl => {}
                WriteRegisters::Audc0 => {}
                WriteRegisters::Audc1 => {}
                WriteRegisters::Audf0 => {}
                WriteRegisters::Audf1 => {}
                WriteRegisters::Audv0 => {}
                WriteRegisters::Audv1 => {}
                WriteRegisters::Grp0 => {
                    if matches!(
                        state_guard.players[0].delay_change_graphic,
                        DelayChangeGraphicPlayer::Disabled
                    ) {
                        state_guard.players[0].graphic = buffer[0];
                    } else {
                        state_guard.players[0].delay_change_graphic =
                            DelayChangeGraphicPlayer::Enabled(Some(buffer[0]));
                    }

                    if let DelayChangeGraphicPlayer::Enabled(Some(graphic)) =
                        state_guard.players[1].delay_change_graphic
                    {
                        state_guard.players[1].graphic = graphic;
                        state_guard.players[1].delay_change_graphic =
                            DelayChangeGraphicPlayer::Enabled(None);
                    }
                }
                WriteRegisters::Grp1 => {
                    if matches!(
                        state_guard.players[1].delay_change_graphic,
                        DelayChangeGraphicPlayer::Disabled
                    ) {
                        state_guard.players[1].graphic = buffer[0];
                    } else {
                        state_guard.players[1].delay_change_graphic =
                            DelayChangeGraphicPlayer::Enabled(Some(buffer[0]));
                    }

                    if let DelayChangeGraphicPlayer::Enabled(Some(graphic)) =
                        state_guard.players[0].delay_change_graphic
                    {
                        state_guard.players[0].graphic = graphic;
                        state_guard.players[0].delay_change_graphic =
                            DelayChangeGraphicPlayer::Enabled(None);
                    }

                    if let DelayEnableChangeBall::Enabled(Some(enabled)) =
                        state_guard.ball.delay_enable_change
                    {
                        state_guard.ball.enabled = enabled;
                        state_guard.ball.delay_enable_change = DelayEnableChangeBall::Enabled(None);
                    }
                }
                WriteRegisters::Enam0 => {
                    state_guard.missiles[0].enabled = data_bits[1];
                }
                WriteRegisters::Enam1 => {
                    state_guard.missiles[1].enabled = data_bits[1];
                }
                WriteRegisters::Enabl => {
                    if matches!(
                        state_guard.ball.delay_enable_change,
                        DelayEnableChangeBall::Disabled
                    ) {
                        state_guard.ball.enabled = data_bits[1];
                    } else {
                        state_guard.ball.delay_enable_change =
                            DelayEnableChangeBall::Enabled(Some(data_bits[1]));
                    }
                }
                WriteRegisters::Hmp0 => {
                    let motion = buffer[0].view_bits::<Msb0>()[0..4].load();

                    state_guard.players[0].motion = motion;
                }
                WriteRegisters::Hmp1 => {
                    let motion = buffer[0].view_bits::<Msb0>()[0..4].load();

                    state_guard.players[1].motion = motion;
                }
                WriteRegisters::Hmm0 => {
                    let motion = buffer[0].view_bits::<Msb0>()[0..4].load();

                    state_guard.missiles[0].motion = motion;
                }
                WriteRegisters::Hmm1 => {
                    let motion = buffer[0].view_bits::<Msb0>()[0..4].load();

                    state_guard.missiles[1].motion = motion;
                }
                WriteRegisters::Hmbl => {
                    let motion = buffer[0].view_bits::<Msb0>()[0..4].load();

                    state_guard.ball.motion = motion;
                }
                WriteRegisters::Vdelp0 => {
                    if data_bits[0] {
                        if matches!(
                            state_guard.players[0].delay_change_graphic,
                            DelayChangeGraphicPlayer::Disabled
                        ) {
                            state_guard.players[0].delay_change_graphic =
                                DelayChangeGraphicPlayer::Enabled(None);
                        }
                    } else {
                        state_guard.players[0].delay_change_graphic =
                            DelayChangeGraphicPlayer::Disabled;
                    }
                }
                WriteRegisters::Vdelp1 => {
                    if data_bits[0] {
                        if matches!(
                            state_guard.players[1].delay_change_graphic,
                            DelayChangeGraphicPlayer::Disabled
                        ) {
                            state_guard.players[1].delay_change_graphic =
                                DelayChangeGraphicPlayer::Enabled(None);
                        }
                    } else {
                        state_guard.players[1].delay_change_graphic =
                            DelayChangeGraphicPlayer::Disabled;
                    }
                }
                WriteRegisters::Vdelbl => {
                    if data_bits[0] {
                        if matches!(
                            state_guard.ball.delay_enable_change,
                            DelayEnableChangeBall::Disabled
                        ) {
                            state_guard.ball.delay_enable_change =
                                DelayEnableChangeBall::Enabled(None);
                        }
                    } else {
                        state_guard.ball.delay_enable_change = DelayEnableChangeBall::Disabled;
                    }
                }
                WriteRegisters::Resmp0 => {
                    if data_bits[1] {
                        state_guard.missiles[0].position = ObjectPosition::LockedToPlayer;
                    }
                }
                WriteRegisters::Resmp1 => {
                    if data_bits[1] {
                        state_guard.missiles[1].position = ObjectPosition::LockedToPlayer;
                    }
                }
                WriteRegisters::Hmove => {}
                WriteRegisters::Hmclr => {
                    state_guard.players[0].motion = 0;
                    state_guard.players[1].motion = 0;
                    state_guard.missiles[0].motion = 0;
                    state_guard.missiles[1].motion = 0;
                    state_guard.ball.motion = 0;
                }
                WriteRegisters::Cxclr => {
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
}

impl ReadMemory for MemoryCallback {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        let state_guard = self.state.lock().unwrap();

        if let Some(register) = ReadRegisters::from_repr(address) {
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
                        .get(&ObjectId::Ball)
                        .map(|set| set.contains(&ObjectId::Playfield))
                        .unwrap_or(false);

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
                ReadRegisters::Inpt0 => {}
                ReadRegisters::Inpt1 => {}
                ReadRegisters::Inpt2 => {}
                ReadRegisters::Inpt3 => {}
                ReadRegisters::Inpt4 => {}
                ReadRegisters::Inpt5 => {}
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
        collision_matrix: &HashMap<ObjectId, HashSet<ObjectId>>,
    ) {
        let collision1 = collision_matrix
            .get(&pair1[0])
            .map(|set| set.contains(&pair1[1]))
            .unwrap_or(false);

        let collision2 = collision_matrix
            .get(&pair2[0])
            .map(|set| set.contains(&pair2[1]))
            .unwrap_or(false);

        let buffer_bits = buffer.view_bits_mut::<Msb0>();

        buffer_bits.set(0, collision1);
        buffer_bits.set(1, collision2);
    }
}
