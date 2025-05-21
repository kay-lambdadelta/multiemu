use super::{MemoryCallback, ReadRegisters};
use crate::tia::{ObjectId, State};
use bitvec::{order::Msb0, view::BitView};
use std::collections::{HashMap, HashSet};

impl MemoryCallback {
    pub(super) fn handle_read_register(
        &self,
        data: &mut u8,
        state_guard: &mut State,
        address: ReadRegisters,
    ) {
        match address {
            ReadRegisters::Cxm0p => {
                self.read_collision_register(
                    data,
                    [ObjectId::Player0, ObjectId::Missile0],
                    [ObjectId::Player1, ObjectId::Missile0],
                    &state_guard.collision_matrix,
                );
            }
            ReadRegisters::Cxm1p => {
                self.read_collision_register(
                    data,
                    [ObjectId::Player0, ObjectId::Missile1],
                    [ObjectId::Player1, ObjectId::Missile1],
                    &state_guard.collision_matrix,
                );
            }
            ReadRegisters::Cxp0fb => {
                self.read_collision_register(
                    data,
                    [ObjectId::Player0, ObjectId::Playfield],
                    [ObjectId::Player0, ObjectId::Ball],
                    &state_guard.collision_matrix,
                );
            }
            ReadRegisters::Cxp1fb => {
                self.read_collision_register(
                    data,
                    [ObjectId::Player1, ObjectId::Playfield],
                    [ObjectId::Player1, ObjectId::Ball],
                    &state_guard.collision_matrix,
                );
            }
            ReadRegisters::Cxm0fb => {
                self.read_collision_register(
                    data,
                    [ObjectId::Missile0, ObjectId::Playfield],
                    [ObjectId::Missile0, ObjectId::Ball],
                    &state_guard.collision_matrix,
                );
            }
            ReadRegisters::Cxm1fb => {
                self.read_collision_register(
                    data,
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

                let data_bits = data.view_bits_mut::<Msb0>();

                data_bits.set(0, collision);
            }
            ReadRegisters::Cxppmm => {
                self.read_collision_register(
                    data,
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
    }

    #[inline]
    fn read_collision_register(
        &self,
        data: &mut u8,
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

        let data_bits = data.view_bits_mut::<Msb0>();

        data_bits.set(0, collision1);
        data_bits.set(1, collision2);
    }
}
