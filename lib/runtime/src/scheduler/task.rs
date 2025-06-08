use crate::scheduler::{RunningState, TaskState};
use num::Zero;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Why the runtime is pausing your execution
pub enum YieldReason {
    /// The runtime is shutting down now
    Exit,
    /// The runtime is pausing your task so it may be kept in real time
    TimeSynchronization,
    /// The runtime is interrupting itself to do some drawing/save stating/input reading/etc. or otherwise things that require locks to be released
    ///
    /// Any locks on state or video buffers should be released here
    RuntimeInterrupt,
}

pub struct SchedulerHandle {
    /// Our shared state with the scheduler
    pub(super) task_state: Arc<TaskState>,
    /// Marks if our thread needs to exit
    pub(super) runtime_shutting_down: Arc<AtomicBool>,
    /// Cycles we can execute before speaking to the scheduler
    pub(super) cycles_until_sync_required: u32,
}

impl SchedulerHandle {
    pub fn tick(&mut self, cleanup_callback: impl FnOnce(YieldReason)) {
        if self.runtime_shutting_down.load(Ordering::Relaxed) {
            cleanup_callback(YieldReason::Exit);
            return;
        }

        if self.cycles_until_sync_required.is_zero() {
            let mut running_state_guard = self.task_state.running_state.lock().unwrap();

            *running_state_guard = RunningState::Waiting;
            drop(running_state_guard);

            // Loop here in case of spurious wakeups
            loop {
                let running_state_guard = self.task_state.running_state.lock().unwrap();

                if let RunningState::CanRun { execution_cycles } = *running_state_guard {
                    self.cycles_until_sync_required = execution_cycles.get();

                    break;
                }
                drop(running_state_guard);

                // Park until the scheduler has assigned us that we can run again
                std::thread::park();
            }
        } else {
            self.cycles_until_sync_required -= 1;
        }
    }
}

pub trait Task: Send + 'static {
    /// Runs in a loop until the runtime says to stop
    fn run(self: Box<Self>, handle: SchedulerHandle);
}

impl<T: FnOnce(SchedulerHandle) + Send + 'static> Task for T {
    fn run(self: Box<Self>, handle: SchedulerHandle) {
        self(handle);
    }
}
