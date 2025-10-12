use super::WriteRegisters;
use crate::tia::{
    DelayChangeGraphicPlayer, DelayEnableChangeBall, InputControl, SCANLINE_LENGTH,
    SupportedGraphicsApiTia, Tia, color::TiaColor, region::Region,
};
use bitvec::{field::BitField, order::Msb0, slice::BitSlice, view::BitView};
use nalgebra::Point2;

const PLAYER_RESP_OFFSET: u16 = 3;
const OTHER_RESP_OFFSET: u16 = 2;

impl<R: Region, G: SupportedGraphicsApiTia> Tia<R, G> {
    pub(crate) fn handle_write_register(
        &mut self,
        data: u8,
        data_bits: &BitSlice<u8>,
        address: WriteRegisters,
    ) {
        match address {
            WriteRegisters::Vsync => {
                if data_bits[1] {
                    self.state.electron_beam = Point2::new(0, 0);
                    self.state.cycles_waiting_for_vsync = Some(SCANLINE_LENGTH * 3);
                } else {
                    if let Some(cycles) = self.state.cycles_waiting_for_vsync
                        && cycles != 0
                    {
                        tracing::warn!("Vsync exited early");
                    }

                    self.state.cycles_waiting_for_vsync = None;
                }
            }
            WriteRegisters::Vblank => {
                self.state.vblank_active = data_bits[1];

                self.state.input_control[0] = if data_bits[7] {
                    InputControl::LatchedOrDumped
                } else {
                    InputControl::Normal
                };

                self.state.input_control[1] = if data_bits[7] {
                    InputControl::LatchedOrDumped
                } else {
                    InputControl::Normal
                };

                self.state.input_control[2] = if data_bits[7] {
                    InputControl::LatchedOrDumped
                } else {
                    InputControl::Normal
                };

                self.state.input_control[3] = if data_bits[7] {
                    InputControl::LatchedOrDumped
                } else {
                    InputControl::Normal
                };

                self.state.input_control[4] = if data_bits[7] {
                    InputControl::LatchedOrDumped
                } else {
                    InputControl::Normal
                };

                self.state.input_control[4] = if data_bits[6] {
                    InputControl::LatchedOrDumped
                } else {
                    InputControl::Normal
                };

                self.state.input_control[5] = if data_bits[6] {
                    InputControl::LatchedOrDumped
                } else {
                    InputControl::Normal
                };
            }
            WriteRegisters::Wsync => {
                self.cpu_rdy.store(false);

                self.state.reset_rdy_on_scanline_end = true;
            }
            WriteRegisters::Rsync => {
                self.state.electron_beam.x = 0;
            }
            WriteRegisters::Nusiz0 => {}
            WriteRegisters::Nusiz1 => {}
            WriteRegisters::Colup0 => {
                let color = extract_color(data_bits);

                self.state.players[0].color = color;
                self.state.missiles[0].color = color;
            }
            WriteRegisters::Colup1 => {
                let color = extract_color(data_bits);

                self.state.players[1].color = color;
                self.state.missiles[1].color = color;
            }
            WriteRegisters::Colupf => {
                let color = extract_color(data_bits);

                self.state.playfield.color = color;
            }
            WriteRegisters::Colubk => {
                let color = extract_color(data_bits);

                self.state.background_color = color;
            }
            WriteRegisters::Ctrlpf => {
                self.state.playfield.mirror = data_bits[0];
                self.state.playfield.score_mode = data_bits[1];
                self.state.high_playfield_ball_priority = data_bits[2];
                self.state.ball.size = 2u8.pow(data_bits[4..=5].load());
            }
            WriteRegisters::Refp0 => {
                self.state.players[0].mirror = data_bits[3];
            }
            WriteRegisters::Refp1 => {
                self.state.players[1].mirror = data_bits[3];
            }
            WriteRegisters::Pf0 => {
                self.state.playfield.data[0..=3].copy_from_bitslice(&data_bits[0..=3]);
            }
            WriteRegisters::Pf1 => {
                self.state.playfield.data[4..=11].copy_from_bitslice(data_bits);
                self.state.playfield.data[4..=11].reverse();
            }
            WriteRegisters::Pf2 => {
                self.state.playfield.data[12..=19].copy_from_bitslice(data_bits);
            }
            WriteRegisters::Resp0 => {
                self.state.players[0].position = self.state.electron_beam.x;
            }
            WriteRegisters::Resp1 => {
                self.state.players[1].position = self.state.electron_beam.x;
            }
            WriteRegisters::Resm0 => {
                self.state.missiles[0].position = self.state.electron_beam.x;
            }
            WriteRegisters::Resm1 => {
                self.state.missiles[1].position = self.state.electron_beam.x;
            }
            WriteRegisters::Resbl => {
                self.state.ball.position = self.state.electron_beam.x;
            }
            WriteRegisters::Audc0 => {}
            WriteRegisters::Audc1 => {}
            WriteRegisters::Audf0 => {}
            WriteRegisters::Audf1 => {}
            WriteRegisters::Audv0 => {}
            WriteRegisters::Audv1 => {}
            WriteRegisters::Grp0 => {
                if matches!(
                    self.state.players[0].delay_change_graphic,
                    DelayChangeGraphicPlayer::Disabled
                ) {
                    self.state.players[0].graphic = data;
                } else {
                    self.state.players[0].delay_change_graphic =
                        DelayChangeGraphicPlayer::Enabled(Some(data));
                }

                if let DelayChangeGraphicPlayer::Enabled(Some(graphic)) =
                    self.state.players[1].delay_change_graphic
                {
                    self.state.players[1].graphic = graphic;
                    self.state.players[1].delay_change_graphic =
                        DelayChangeGraphicPlayer::Enabled(None);
                }
            }
            WriteRegisters::Grp1 => {
                if matches!(
                    self.state.players[1].delay_change_graphic,
                    DelayChangeGraphicPlayer::Disabled
                ) {
                    self.state.players[1].graphic = data;
                } else {
                    self.state.players[1].delay_change_graphic =
                        DelayChangeGraphicPlayer::Enabled(Some(data));
                }

                if let DelayChangeGraphicPlayer::Enabled(Some(graphic)) =
                    self.state.players[0].delay_change_graphic
                {
                    self.state.players[0].graphic = graphic;
                    self.state.players[0].delay_change_graphic =
                        DelayChangeGraphicPlayer::Enabled(None);
                }

                if let DelayEnableChangeBall::Enabled(Some(enabled)) =
                    self.state.ball.delay_enable_change
                {
                    self.state.ball.enabled = enabled;
                    self.state.ball.delay_enable_change = DelayEnableChangeBall::Enabled(None);
                }
            }
            WriteRegisters::Enam0 => {
                self.state.missiles[0].enabled = data_bits[1];
            }
            WriteRegisters::Enam1 => {
                self.state.missiles[1].enabled = data_bits[1];
            }
            WriteRegisters::Enabl => {
                if matches!(
                    self.state.ball.delay_enable_change,
                    DelayEnableChangeBall::Disabled
                ) {
                    self.state.ball.enabled = data_bits[1];
                } else {
                    self.state.ball.delay_enable_change =
                        DelayEnableChangeBall::Enabled(Some(data_bits[1]));
                }
            }
            WriteRegisters::Hmp0 => {
                self.state.players[0].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmp1 => {
                self.state.players[1].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmm0 => {
                self.state.missiles[0].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmm1 => {
                self.state.missiles[1].motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Hmbl => {
                self.state.ball.motion = data.view_bits::<Msb0>()[0..4].load();
            }
            WriteRegisters::Vdelp0 => {
                if data_bits[0] {
                    if matches!(
                        self.state.players[0].delay_change_graphic,
                        DelayChangeGraphicPlayer::Disabled
                    ) {
                        self.state.players[0].delay_change_graphic =
                            DelayChangeGraphicPlayer::Enabled(None);
                    }
                } else {
                    self.state.players[0].delay_change_graphic = DelayChangeGraphicPlayer::Disabled;
                }
            }
            WriteRegisters::Vdelp1 => {
                if data_bits[0] {
                    if matches!(
                        self.state.players[1].delay_change_graphic,
                        DelayChangeGraphicPlayer::Disabled
                    ) {
                        self.state.players[1].delay_change_graphic =
                            DelayChangeGraphicPlayer::Enabled(None);
                    }
                } else {
                    self.state.players[1].delay_change_graphic = DelayChangeGraphicPlayer::Disabled;
                }
            }
            WriteRegisters::Vdelbl => {
                if data_bits[0] {
                    if matches!(
                        self.state.ball.delay_enable_change,
                        DelayEnableChangeBall::Disabled
                    ) {
                        self.state.ball.delay_enable_change = DelayEnableChangeBall::Enabled(None);
                    }
                } else {
                    self.state.ball.delay_enable_change = DelayEnableChangeBall::Disabled;
                }
            }
            WriteRegisters::Resmp0 => {
                self.state.missiles[0].locked = data_bits[1];
            }
            WriteRegisters::Resmp1 => {
                self.state.missiles[1].locked = data_bits[1];
            }
            WriteRegisters::Hmove => {
                for player in &mut self.state.players {
                    player.position = player.position.wrapping_add_signed(player.motion as i16);
                }

                for missile in &mut self.state.missiles {
                    missile.position = missile.position.wrapping_add_signed(missile.motion as i16);
                }

                self.state.ball.position = self
                    .state
                    .ball
                    .position
                    .wrapping_add_signed(self.state.ball.motion as i16);
            }
            WriteRegisters::Hmclr => {
                self.state.players[0].motion = 0;
                self.state.players[1].motion = 0;
                self.state.missiles[0].motion = 0;
                self.state.missiles[1].motion = 0;
                self.state.ball.motion = 0;
            }
            WriteRegisters::Cxclr => {
                self.state.collision_matrix.clear();
            }
        }
    }
}

fn extract_color(data_bits: &BitSlice<u8>) -> TiaColor {
    let luminance = data_bits[1..=3].load();
    let hue = data_bits[4..=7].load();

    TiaColor { luminance, hue }
}
