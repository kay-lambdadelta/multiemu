use super::{ReadRegisters, State, WriteRegisters};
use crate::tia::{InputControl, ObjectId, ObjectPosition};
use bitvec::{
    order::{Lsb0, Msb0},
    view::BitView,
};
use multiemu_definition_m6502::M6502;
use multiemu_machine::{
    component::component_ref::ComponentRef,
    memory::{
        AddressSpaceId,
        callbacks::Memory,
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use petgraph::prelude::UnGraphMap;
use rangemap::RangeInclusiveMap;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct MemoryCallback {
    pub state: Arc<Mutex<State>>,
    pub processor: ComponentRef<M6502>,
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
            tracing::debug!("Writing to TIA register: {:?} = {:02x}", address, buffer[0]);

            match address {
                WriteRegisters::Vsync => {
                    state_guard.in_vsync = data[1];
                }
                WriteRegisters::Vblank => {
                    state_guard.in_vblank = data[1];

                    state_guard.input_control[0] = if data[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[1] = if data[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[2] = if data[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[3] = if data[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[4] = if data[7] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[4] = if data[6] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };

                    state_guard.input_control[5] = if data[6] {
                        InputControl::LatchedOrDumped
                    } else {
                        InputControl::Normal
                    };
                }
                WriteRegisters::Wsync => {
                    self.processor
                        .interact(|processor| processor.set_rdy(false));
                    state_guard.reset_rdy_on_scanline_end = true;
                }
                WriteRegisters::Rsync => {}
                WriteRegisters::Nusiz0 => {}
                WriteRegisters::Nusiz1 => {}
                WriteRegisters::Colup0 => {}
                WriteRegisters::Colup1 => {}
                WriteRegisters::Colupf => {}
                WriteRegisters::Colubk => {}
                WriteRegisters::Ctrlpf => {}
                WriteRegisters::Refp0 => {}
                WriteRegisters::Refp1 => {}
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
                WriteRegisters::Grp0 => {}
                WriteRegisters::Grp1 => {}
                WriteRegisters::Enam0 => {}
                WriteRegisters::Enam1 => {}
                WriteRegisters::Enabl => {}
                WriteRegisters::Hmp0 => {}
                WriteRegisters::Hmp1 => {}
                WriteRegisters::Hmm0 => {}
                WriteRegisters::Hmm1 => {}
                WriteRegisters::Hmbl => {}
                WriteRegisters::Vdelp0 => {}
                WriteRegisters::Vdelp1 => {}
                WriteRegisters::Vdelpl => {}
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
