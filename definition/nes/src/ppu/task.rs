use crate::ppu::NesPpu;
use multiemu_runtime::{component::ComponentRef, scheduler::Task};
use std::num::NonZero;

pub struct PpuDriver {
    component: ComponentRef<NesPpu>,
}

impl Task for PpuDriver {
    fn run(&mut self, time_slice: NonZero<u32>) {
        self.component.interact_mut(|component| {}).unwrap();
    }
}
