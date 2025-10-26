use crate::component::Component;
use std::fmt::Debug;
use std::num::NonZero;

pub trait Task<C: Component>: Send + Sync + 'static {
    /// Runs in a loop until the runtime says to stop
    fn run(&mut self, component: &mut C, time_slice: NonZero<u32>);
}

impl<C: Component, T: FnMut(&mut C, NonZero<u32>) + Send + Sync + 'static> Task<C> for T {
    #[inline]
    fn run(&mut self, component: &mut C, time_slice: NonZero<u32>) {
        self(component, time_slice)
    }
}

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

pub type TaskId = u16;
pub type ErasedTask = Box<dyn FnMut(&mut dyn Component, NonZero<u32>) + Send + Sync>;

pub struct TaskData {
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
