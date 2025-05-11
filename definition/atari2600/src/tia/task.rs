use super::{SCANLINE_LENGTH, Tia, region::Region};
use multiemu_definition_mos6502::Mos6502;
use multiemu_machine::{component::component_ref::ComponentRef, scheduler::task::Task};
use nalgebra::Point2;
use std::num::NonZero;

pub struct TiaTask {
    pub processor: ComponentRef<Mos6502>,
}

impl<R: Region> Task<Tia<R>> for TiaTask {
    fn run(&mut self, target: &Tia<R>, period: NonZero<u32>) {
        let period = period.get();
        let mut state_guard = target.state.lock().unwrap();

        for _ in 0..period {
            state_guard.horizontal_timer += 1;

            if state_guard.horizontal_timer >= SCANLINE_LENGTH {
                state_guard.horizontal_timer = 0;
                state_guard.scanline += 1;

                if std::mem::replace(&mut state_guard.reset_rdy_on_scanline_end, false) {
                    self.processor
                        .interact(|processor| {
                            processor.set_rdy(true);
                        })
                        .unwrap();
                }
            }

            if state_guard.scanline >= R::TOTAL_SCANLINES {
                state_guard.scanline = 0;
            }

            target.display_backend.get().unwrap().draw(
                &state_guard,
                Point2::new(state_guard.horizontal_timer as u16, state_guard.scanline),
                3,
                7,
            );
        }
    }
}
