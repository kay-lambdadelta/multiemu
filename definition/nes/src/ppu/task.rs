use crate::ppu::{Ppu, region::Region};
use multiemu_runtime::{component::ComponentRef, scheduler::Task};
use std::num::NonZero;

pub struct PpuDriver<R: Region> {
    pub component: ComponentRef<Ppu<R>>,
}

impl<R: Region> Task for PpuDriver<R> {
    fn run(&mut self, time_slice: NonZero<u32>) {
        self.component
            .interact_mut(|component| {
                let mut state_guard = component.state.lock().unwrap();

                for _ in 0..time_slice.get() {
                    state_guard.electron_beam.x += 1;

                    if state_guard.electron_beam.x >= R::visible_scanline_dimensions().x {
                        state_guard.electron_beam.x = 0;
                        state_guard.electron_beam.y += 1;

                        if state_guard.electron_beam.y >= R::visible_scanline_dimensions().y {
                            state_guard.electron_beam.y = 0;
                        }
                    }
                }
            })
            .unwrap();
    }
}
