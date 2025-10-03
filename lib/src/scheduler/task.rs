use crate::component::Component;
use std::num::NonZero;

pub trait Task<C: Component>: Send + Sync + 'static {
    /// Runs in a loop until the runtime says to stop
    fn run(&mut self, component: &C, time_slice: NonZero<u32>);
}

impl<C: Component, T: FnMut(&C, NonZero<u32>) + Send + Sync + 'static> Task<C> for T {
    #[inline]
    fn run(&mut self, component: &C, time_slice: NonZero<u32>) {
        self(component, time_slice)
    }
}

pub trait TaskMut<C: Component>: Send + Sync + 'static {
    /// Runs in a loop until the runtime says to stop
    fn run(&mut self, component: &mut C, time_slice: NonZero<u32>);
}

impl<C: Component, T: FnMut(&mut C, NonZero<u32>) + Send + Sync + 'static> TaskMut<C> for T {
    #[inline]
    fn run(&mut self, component: &mut C, time_slice: NonZero<u32>) {
        self(component, time_slice)
    }
}
