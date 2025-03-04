use crate::component::Component;

pub trait Task<C: Component>: Send + 'static {
    fn run(&mut self, target: &C, period: u64);
}

impl<C: Component, T: FnMut(&C, u64) + Send + 'static> Task<C> for T {
    fn run(&mut self, target: &C, period: u64) {
        self(target, period);
    }
}
