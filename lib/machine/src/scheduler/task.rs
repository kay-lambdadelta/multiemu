use crate::component::Component;

pub trait Task<C: Component>: 'static {
    fn run(&mut self, target: &C, period: u64);
}

impl<C: Component, T: FnMut(&C, u64) + 'static> Task<C> for T {
    fn run(&mut self, target: &C, period: u64) {
        self(target, period);
    }
}
