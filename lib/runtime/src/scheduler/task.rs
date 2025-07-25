use std::num::NonZero;

/// A task to run
pub trait Task: Send + Sync + 'static {
    /// Runs in a loop until the runtime says to stop
    fn run(&mut self, time_slice: NonZero<u32>);
}

impl<T: FnMut(NonZero<u32>) + Send + Sync + 'static> Task for T {
    fn run(&mut self, time_slice: NonZero<u32>) {
        self(time_slice)
    }
}
