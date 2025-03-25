use crate::component::Component;
use std::num::NonZero;

pub trait Task<C: Component>: Send + 'static {
    fn run(&mut self, target: &C, period: NonZero<u32>);
}

impl<C: Component, T: FnMut(&C, NonZero<u32>) + Send + 'static> Task<C> for T {
    fn run(&mut self, target: &C, period: NonZero<u32>) {
        self(target, period);
    }
}
