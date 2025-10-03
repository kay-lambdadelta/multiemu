use super::{SCANLINE_LENGTH, State, Tia, color::TiaColor, region::Region};
use crate::tia::{
    VISIBLE_SCANLINE_LENGTH,
    backend::{SupportedGraphicsApiTia, TiaDisplayBackend},
};
use bitvec::{
    order::{Lsb0, Msb0},
    view::BitView,
};
use multiemu::scheduler::TaskMut;
use multiemu_definition_mos6502::RdyFlag;
use std::{num::NonZero, sync::Arc};

pub struct TiaTask {
    pub cpu_rdy: Arc<RdyFlag>,
}

impl<R: Region, G: SupportedGraphicsApiTia> TaskMut<Tia<R, G>> for TiaTask {
    fn run(&mut self, component: &mut Tia<R, G>, time_slice: NonZero<u32>) {
        let mut commit_staging_buffer = false;
        let backend_guard = component.backend.as_mut().unwrap();

        for _ in 0..time_slice.get() {
            if let Some(cycles) = component.state.cycles_waiting_for_vsync {
                component.state.cycles_waiting_for_vsync = Some(cycles.saturating_sub(1));

                if component.state.cycles_waiting_for_vsync == Some(0) {
                    commit_staging_buffer = true;
                }
            }

            if !(component.state.cycles_waiting_for_vsync.is_some()
                || component.state.vblank_active)
                && (0..VISIBLE_SCANLINE_LENGTH).contains(&component.state.electron_beam.x)
            {
                let color = R::color_to_srgb(component.state.get_rendered_color());

                backend_guard.modify_staging_buffer(|mut staging_buffer_guard| {
                    staging_buffer_guard[(
                        component.state.electron_beam.x as usize,
                        component.state.electron_beam.y as usize,
                    )] = color.into();
                });
            }

            component.state.electron_beam.x += 1;

            if component.state.electron_beam.x >= SCANLINE_LENGTH {
                component.state.electron_beam.x = 0;
                component.state.electron_beam.y += 1;

                if std::mem::replace(&mut component.state.reset_rdy_on_scanline_end, false) {
                    self.cpu_rdy.store(true);
                }

                if component.state.electron_beam.y >= R::TOTAL_SCANLINES {
                    component.state.electron_beam.y = 0;
                }
            }
        }

        if commit_staging_buffer {
            backend_guard.commit_staging_buffer();
        }
    }
}

impl State {
    fn get_rendered_color(&self) -> TiaColor {
        if self.high_playfield_ball_priority {
            // Check if in the bounds of ball
            if self.get_ball_color() {
                return self.ball.color;
            }

            // Check if in the bounds of playfield
            if let Some(color) = self.get_playfield_color() {
                return color;
            }

            // Check if in the bounds of player 0
            if let Some(color) = self.get_player_color(0) {
                return color;
            }

            // Check if in the bounds of player 1
            if let Some(color) = self.get_player_color(1) {
                return color;
            }

            // Check if in the bounds of missile 0
            if self.get_missile_color(0) {
                return self.missiles[0].color;
            }

            // Check if in the bounds of missile 1
            if self.get_missile_color(1) {
                return self.missiles[1].color;
            }
        } else {
            // Check if in the bounds of player 0
            if let Some(color) = self.get_player_color(0) {
                return color;
            }

            // Check if in the bounds of player 1
            if let Some(color) = self.get_player_color(1) {
                return color;
            }

            // Check if in the bounds of missile 0
            if self.get_missile_color(0) {
                return self.missiles[0].color;
            }

            // Check if in the bounds of missile 1
            if self.get_missile_color(1) {
                return self.missiles[1].color;
            }

            // Check if in the bounds of ball
            if self.get_ball_color() {
                return self.ball.color;
            }

            // Check if in the bounds of playfield
            if let Some(color) = self.get_playfield_color() {
                return color;
            }
        }

        self.background_color
    }

    #[inline]
    fn get_player_color(&self, index: usize) -> Option<TiaColor> {
        let player = &self.players[index];

        if let Some(sprite_pixel) = self
            .electron_beam
            .x
            .checked_sub(player.position)
            .map(usize::from)
        {
            if player.mirror {
                let slice = player.graphic.view_bits::<Lsb0>();

                if let Some(sprite_pixel) = slice.get(sprite_pixel).as_deref() {
                    return if *sprite_pixel {
                        Some(player.color)
                    } else {
                        None
                    };
                }
            } else {
                let slice = player.graphic.view_bits::<Msb0>();

                if let Some(sprite_pixel) = slice.get(sprite_pixel).as_deref() {
                    return if *sprite_pixel {
                        Some(player.color)
                    } else {
                        None
                    };
                }
            }
        }

        None
    }

    #[inline]
    fn get_missile_color(&self, index: usize) -> bool {
        let missile = &self.missiles[index];

        if missile.locked {
            return false;
        }

        self.electron_beam.x == missile.position
    }

    #[inline]
    fn get_ball_color(&self) -> bool {
        let ball = &self.ball;

        self.electron_beam.x == ball.position
    }

    #[inline]
    fn get_playfield_color(&self) -> Option<TiaColor> {
        let playfield_position = (self.electron_beam.x / 4) as usize;

        match playfield_position {
            0..20 => {
                if self.playfield.data[playfield_position] {
                    if self.playfield.score_mode {
                        Some(self.players[0].color)
                    } else {
                        Some(self.playfield.color)
                    }
                } else {
                    None
                }
            }
            20..40 => {
                let mut data = self.playfield.data;

                if self.playfield.mirror {
                    data.reverse();
                }

                if data[playfield_position - 20] {
                    if self.playfield.score_mode {
                        Some(self.players[1].color)
                    } else {
                        Some(self.playfield.color)
                    }
                } else {
                    None
                }
            }
            _ => unreachable!(),
        }
    }
}
