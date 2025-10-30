use crate::component::Component;
use std::fmt::Debug;
use std::num::NonZero;

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

/// The type of task that a [Task] represents
///
/// This affects scheduler behavior greatly
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskType {
    /// Tasks are run in their ordering as specified by their frequency
    Direct,
    /// Tasks are run when their component is interacted with, greatly increasing performance
    ///
    /// NOTE: This pretty much destroys determinism with component tasks that write to other components or shared state
    /// Use with caution, but determinism is preserved when component tasks only write to themselves/their component
    Lazy,
}

pub(crate) type TaskId = u16;
pub(crate) type ErasedTask = Box<dyn FnMut(&mut dyn Component, NonZero<u32>) + Send + Sync>;

pub(crate) struct TaskData {
    // Pointer to callable task
    pub callback: ErasedTask,
    // Debt this component accumulated
    pub debt: u32,
    /// Type of task
    pub ty: TaskType,
}

impl Debug for TaskData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskData")
            .field("debt", &self.debt)
            .field("ty", &self.ty)
            .finish()
    }
}
