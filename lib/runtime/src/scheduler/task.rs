use std::{fmt::Debug, num::NonZero};

use crate::component::Component;

/// A callback that the scheduler executes to drive component state in a frequency based manner
pub trait Task<C: Component>: Send + Sync + 'static {
    /// Given the component, advance the state machine by these many cycles
    ///
    /// Note that the time slice argument is not a suggestion, components must literally satisfy the timing requirements
    /// for deterministic execution to operate correctly
    fn run(&mut self, component: &mut C, time_slice: NonZero<u32>);
}

// Blanket impl for closures and functions

impl<C: Component, T: FnMut(&mut C, NonZero<u32>) + Send + Sync + 'static> Task<C> for T {
    #[inline]
    fn run(&mut self, component: &mut C, time_slice: NonZero<u32>) {
        self(component, time_slice);
    }
}

pub(crate) type TaskId = u16;
pub(crate) type ErasedTask = Box<dyn FnMut(&mut dyn Component, NonZero<u32>) + Send + Sync>;

pub(crate) struct TaskData {
    // Pointer to callable task
    pub callback: ErasedTask,
}

impl Debug for TaskData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskData").finish()
    }
}
