use num::Zero;

use crate::scheduler::{RunningState, TaskState};
use std::{
    ops::DerefMut,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
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
    pub(super) task_state: Arc<TaskState>,
    pub(super) runtime_shutting_down: Arc<AtomicBool>,
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

            // Show we are waiting
            *running_state_guard = RunningState::Waiting;

            cleanup_callback(YieldReason::TimeSynchronization);

            // Make sure the runtime is notified
            self.task_state.condvar.notify_one();
            let mut running_state_guard = self
                .task_state
                .condvar
                .wait_while(running_state_guard, |is_ready| {
                    // Wait for the runtime to give us the go ahead
                    !matches!(*is_ready, RunningState::CanRun(_))
                })
                .unwrap();

            // Get our things
            let RunningState::CanRun(time_slice) =
                std::mem::replace(running_state_guard.deref_mut(), RunningState::Running)
            else {
                unreachable!()
            };

            self.cycles_until_sync_required = time_slice.get();
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
