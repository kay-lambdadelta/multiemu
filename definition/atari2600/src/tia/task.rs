use std::num::NonZero;

use crate::tia::{
    VISIBLE_SCANLINE_LENGTH,
    backend::{SupportedGraphicsApiTia, TiaDisplayBackend},
};

use super::{SCANLINE_LENGTH, State, Tia, color::TiaColor, region::Region};
use bitvec::{
    order::{Lsb0, Msb0},
    view::BitView,
};
use multiemu_definition_mos6502::Mos6502;
use multiemu_runtime::{component::ComponentRef, scheduler::Task};

pub struct TiaTask<R: Region, G: SupportedGraphicsApiTia> {
    pub component: ComponentRef<Tia<R, G>>,
    pub cpu: ComponentRef<Mos6502>,
}

impl<R: Region, G: SupportedGraphicsApiTia> Task for TiaTask<R, G> {
    fn run(&mut self, time_slice: NonZero<u32>) {
        // This task will usually get called with a time slice of 3 since its 3 times faster than the cpu and the fastest timer in the atari 2600
        let mut pixels = Vec::with_capacity(time_slice.get() as usize);

        self.component
            .interact(|component| {
                let mut state_guard = component.state.lock().unwrap();
                let mut commit_staging_buffer = false;
                let mut backend_guard = component.backend.lock().unwrap();

                for _ in 0..time_slice.get() {
                    if let Some(cycles) = state_guard.cycles_waiting_for_vsync {
                        state_guard.cycles_waiting_for_vsync = Some(cycles.saturating_sub(1));

                        if state_guard.cycles_waiting_for_vsync == Some(0) {
                            commit_staging_buffer = true;
                        }
                    }

                    state_guard.electron_beam.x += 1;

                    if state_guard.electron_beam.x >= SCANLINE_LENGTH {
                        state_guard.electron_beam.x = 0;
                        state_guard.electron_beam.y += 1;

                        if std::mem::replace(&mut state_guard.reset_rdy_on_scanline_end, false) {
                            self.cpu
                                .interact(|processor| {
                                    processor.set_rdy(true);
                                })
                                .unwrap();
                        }

                        if state_guard.electron_beam.y >= R::TOTAL_SCANLINES {
                            state_guard.electron_beam.y = 0;
                        }
                    }

                    if !(state_guard.cycles_waiting_for_vsync.is_some()
                        || state_guard.vblank_active)
                        && (0..VISIBLE_SCANLINE_LENGTH).contains(&state_guard.electron_beam.x)
                    {
                        let color = R::color_to_srgb(state_guard.get_rendered_color());

                        pixels.push((color, state_guard.electron_beam));
                    }
                }

                backend_guard.modify_staging_buffer(|mut staging_buffer_guard| {
                    for (color, position) in pixels {
                        staging_buffer_guard[(position.x as usize, position.y as usize)] =
                            color.into();
                    }
                });

                if commit_staging_buffer {
                    backend_guard.commit_staging_buffer();
                }
            })
            .unwrap();
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
                        Some(self.background_color)
                    };
                }
            } else {
                let slice = player.graphic.view_bits::<Msb0>();

                if let Some(sprite_pixel) = slice.get(sprite_pixel).as_deref() {
                    return if *sprite_pixel {
                        Some(player.color)
                    } else {
                        Some(self.background_color)
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

        (self.electron_beam.x..=(self.electron_beam.x)).contains(&missile.position)
    }

    #[inline]
    fn get_ball_color(&self) -> bool {
        let ball = &self.ball;

        (self.electron_beam.x..=(self.electron_beam.x)).contains(&ball.position)
    }

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
                    Some(self.background_color)
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
                    Some(self.background_color)
                }
            }
            _ => unreachable!(),
        }
    }
}
