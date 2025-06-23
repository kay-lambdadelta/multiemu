use super::WriteRegisters;
use crate::tia::{
    DelayChangeGraphicPlayer, DelayEnableChangeBall, InputControl, State, SupportedGraphicsApiTia,
    Tia, region::Region,
};
use bitvec::{field::BitField, order::Msb0, slice::BitSlice, view::BitView};

impl<R: Region, G: SupportedGraphicsApiTia> Tia<R, G> {
    pub(crate) fn handle_write_register(
        &self,
        data: u8,
        data_bits: &BitSlice<u8>,
        state_guard: &mut State,
        address: WriteRegisters,
    ) {
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
                self.config
                    .cpu
                    .interact(|processor| processor.set_rdy(false))
                    .unwrap();
                state_guard.reset_rdy_on_scanline_end = true;
            }
            WriteRegisters::Rsync => {
                state_guard.electron_beam.x = 0;
            }
            WriteRegisters::Nusiz0 => {}
            WriteRegisters::Nusiz1 => {}
            WriteRegisters::Colup0 => {}
            WriteRegisters::Colup1 => {}
            WriteRegisters::Colupf => {}
            WriteRegisters::Colubk => {}
            WriteRegisters::Ctrlpf => {
                state_guard.playfield.mirror = data_bits[0];
                state_guard.playfield.score_mode = data_bits[1];
                state_guard.high_playfield_ball_priority = data_bits[2];
                state_guard.ball.size = 2u8.pow(data_bits[4..=5].load());
            }
            WriteRegisters::Refp0 => {
                state_guard.players[0].mirror = data_bits[3];
            }
            WriteRegisters::Refp1 => {
                state_guard.players[1].mirror = data_bits[3];
            }
            WriteRegisters::Pf0 => {
                state_guard.playfield.data[0..=3].copy_from_bitslice(&data_bits[0..=3]);
            }
            WriteRegisters::Pf1 => {
                state_guard.playfield.data[4..=11].copy_from_bitslice(data_bits);
            }
            WriteRegisters::Pf2 => {
                state_guard.playfield.data[12..=19].copy_from_bitslice(data_bits);
            }
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
                    state_guard.players[0].graphic = data;
                } else {
                    state_guard.players[0].delay_change_graphic =
                        DelayChangeGraphicPlayer::Enabled(Some(data));
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
                    state_guard.players[1].graphic = data;
                } else {
                    state_guard.players[1].delay_change_graphic =
                        DelayChangeGraphicPlayer::Enabled(Some(data));
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
                state_guard.players[0].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmp1 => {
                state_guard.players[1].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmm0 => {
                state_guard.missiles[0].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmm1 => {
                state_guard.missiles[1].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmbl => {
                state_guard.ball.motion = data.view_bits::<Msb0>()[0..4].load();
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
                        state_guard.ball.delay_enable_change = DelayEnableChangeBall::Enabled(None);
                    }
                } else {
                    state_guard.ball.delay_enable_change = DelayEnableChangeBall::Disabled;
                }
            }
            WriteRegisters::Resmp0 => {
                state_guard.missiles[0].locked = data_bits[1];
            }
            WriteRegisters::Resmp1 => {
                state_guard.missiles[1].locked = data_bits[1];
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
    }
}
