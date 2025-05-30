use super::{
    SCANLINE_LENGTH, State, SupportedRenderApiTia, Tia, TiaDisplayBackend, color::TiaColor,
    region::Region,
};
use bitvec::{
    order::{Lsb0, Msb0},
    view::BitView,
};
use multiemu_definition_mos6502::Mos6502;
use multiemu_machine::{component::component_ref::ComponentRef, task::Task};
use std::num::NonZero;

pub struct TiaTask {
    pub cpu: ComponentRef<Mos6502>,
}

impl<R: Region, A: SupportedRenderApiTia> Task<Tia<R, A>> for TiaTask {
    fn run(&mut self, target: &Tia<R, A>, period: NonZero<u32>) {
        let period = period.get();
        let mut state_guard = target.state.lock().unwrap();

        for _ in 0..period {
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
            }

            if state_guard.electron_beam.y >= R::TOTAL_SCANLINES {
                state_guard.electron_beam.y = 0;
            }

            target
                .display_backend
                .get()
                .unwrap()
                .draw(state_guard.electron_beam, state_guard.get_rendered_color());
        }
    }
}

impl State {
    fn get_rendered_color(&self) -> TiaColor {
        if self.high_playfield_ball_priority {
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
        }

        TiaColor::default()
    }

    #[inline]
    fn get_player_color(&self, index: usize) -> Option<TiaColor> {
        let player = &self.players[index];

        if let Some(sprite_pixel) = self
            .electron_beam
            .x
            .checked_sub(player.position.x)
            .map(usize::from)
        {
            if self.electron_beam.y == player.position.y {
                if player.mirror {
                    let slice = player.graphic.view_bits::<Lsb0>();

                    if *slice.get(sprite_pixel).as_deref().unwrap_or(&false) {
                        return Some(player.color);
                    }
                } else {
                    let slice = player.graphic.view_bits::<Msb0>();

                    if *slice.get(sprite_pixel).as_deref().unwrap_or(&false) {
                        return Some(player.color);
                    }
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

        (self.electron_beam.x..=(self.electron_beam.x)).contains(&missile.position.x)
            && self.electron_beam.y == missile.position.y
    }

    #[inline]
    fn get_ball_color(&self) -> bool {
        let ball = &self.ball;

        (self.electron_beam.x..=(self.electron_beam.x)).contains(&ball.position.x)
            && self.electron_beam.y == ball.position.y
    }
}
