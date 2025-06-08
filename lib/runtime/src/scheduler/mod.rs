use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
use std::{
    boxed::Box,
    cmp::Reverse,
    collections::BinaryHeap,
    fmt::Debug,
    num::NonZero,
    sync::{
        Arc, Barrier, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::Thread,
    time::Duration,
    vec::Vec,
};

mod run;
mod task;

pub use task::*;

use crate::utils::MainThreadQueue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
enum RunningState {
    CanRun {
        execution_cycles: NonZero<u32>,
    },
    #[default]
    Waiting,
}

#[derive(Debug, Default)]
struct TaskState {
    pub running_state: Mutex<RunningState>,
}

#[derive(Debug)]
struct TaskInfo {
    pub task_state: Arc<TaskState>,
    pub tick_rate: u32,
    pub next_execution: Reverse<u32>,
    pub thread: Thread,
}

impl PartialEq for TaskInfo {
    fn eq(&self, other: &Self) -> bool {
        self.next_execution == other.next_execution
    }
}

impl PartialOrd for TaskInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for TaskInfo {}

impl Ord for TaskInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.next_execution.cmp(&other.next_execution)
    }
}

#[derive(Debug)]
pub struct Scheduler {
    /// Global tick we are currently on
    current_tick: u32,
    /// Rollover tick
    rollover_tick: u32,
    /// The amount of time a tick takes
    tick_real_time: Duration,
    /// Amount of time we can execute per pass
    allotted_time: Duration,
    /// Tasks sorted in a min heap
    tasks: BinaryHeap<TaskInfo>,
    /// Temporary storage for tasks
    to_run: Vec<TaskInfo>,
    /// Handle to the main thread queue
    main_thread_queue: Arc<MainThreadQueue>,
    /// Flag to tell if the runtime is going away
    runtime_shutting_down: Arc<AtomicBool>,
    /// Barrier to block tasks from running until the first pass
    barrier: Option<Arc<Barrier>>,
    /// Tasks that are currently executing
    inflight: Vec<Arc<TaskState>>,
}

impl Scheduler {
    pub(crate) fn new(
        tasks: impl IntoIterator<Item = (Ratio<u32>, Box<dyn Task>)>,
        main_thread_queue: Arc<MainThreadQueue>,
    ) -> Self {
        pub struct PrecalculationTask {
            #[allow(clippy::type_complexity)]
            pub task: Box<dyn Task>,
            pub frequency: Ratio<u32>,
        }

        let runtime_shutting_down = Arc::new(AtomicBool::new(false));

        let tasks: Vec<_> = tasks
            .into_iter()
            .enumerate()
            .map(|(task_id, (frequency, task))| {
                tracing::debug!(
                    "Task {} has a frequency of {} (period of {:?})",
                    task_id,
                    frequency,
                    Duration::from_secs_f64(frequency.recip().to_f64().unwrap())
                );

                PrecalculationTask {
                    task,
                    frequency: frequency.recip(),
                }
            })
            .collect();

        // The number of tasks + the driving thread
        let barrier = Arc::new(Barrier::new(tasks.len() + 1));

        let common = Ratio::new(
            tasks
                .iter()
                .map(|task| *task.frequency.numer())
                .fold(1, gcd),
            tasks
                .iter()
                .map(|task| *task.frequency.denom())
                .fold(1, lcm),
        );

        tracing::info!("System frequency is {}", common);

        let tick_real_time = Duration::from_secs_f64(common.to_f64().unwrap());
        let rollover_tick = common.recip().to_integer();

        tracing::debug!(
            "Schedule ticks take {:?} and rolls over at tick {}, a full cycle takes {:?}",
            tick_real_time,
            rollover_tick,
            tick_real_time * rollover_tick as u32
        );

        let tasks: BinaryHeap<TaskInfo> = tasks
            .into_iter()
            .map(|precalcuation_task| {
                let factor = rollover_tick / precalcuation_task.frequency.denom();
                let tick_rate = precalcuation_task.frequency.numer() * factor;

                let task_state = Arc::new(TaskState::default());
                let scheduler_handle = SchedulerHandle {
                    task_state: task_state.clone(),
                    runtime_shutting_down: runtime_shutting_down.clone(),
                    cycles_until_sync_required: 0,
                };
                let barrier = barrier.clone();

                let join_handle = {
                    let task = precalcuation_task.task;

                    std::thread::spawn(move || {
                        barrier.wait();
                        drop(barrier);

                        task.run(scheduler_handle);
                    })
                };

                TaskInfo {
                    task_state,
                    tick_rate,
                    next_execution: Reverse(0),
                    thread: join_handle.thread().clone(),
                }
            })
            .collect();

        tracing::debug!("Tasks {:#?}", tasks);

        Self {
            current_tick: 0,
            tick_real_time,
            rollover_tick,
            allotted_time: Duration::from_secs(1) / 60,
            tasks,
            to_run: Vec::default(),
            runtime_shutting_down,
            main_thread_queue,
            barrier: Some(barrier),
            inflight: Vec::default(),
        }
    }

    pub fn slow_down(&mut self) {
        // Set our allotted time to lower but not lower than one tick
        self.allotted_time = self
            .allotted_time
            .saturating_sub(Duration::from_nanos(500))
            .max(self.tick_real_time);

        tracing::trace!(
            "Alotted time for scheduler moved down to {:?}",
            self.allotted_time
        );
    }

    pub fn speed_up(&mut self) {
        // Set our allotted time higher but not higher than what one period takes
        self.allotted_time = self
            .allotted_time
            .saturating_add(Duration::from_nanos(500))
            .min(self.tick_real_time * self.rollover_tick);

        tracing::trace!(
            "Alotted time for scheduler moved up to {:?}",
            self.allotted_time
        );
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.runtime_shutting_down.store(true, Ordering::Relaxed);
    }
}

#[inline]
fn run_task(
    to_run: impl IntoIterator<Item = (TaskInfo, NonZero<u32>)>,
    heap: &mut BinaryHeap<TaskInfo>,
    inflight: &mut Vec<Arc<TaskState>>,
    main_thread_queue: &MainThreadQueue,
    tick_real_time: Duration,
) {
    wait_on_inflight(inflight, main_thread_queue, tick_real_time);

    for (mut task_info, time_slice) in to_run {
        let mut running_state_guard = task_info.task_state.running_state.lock().unwrap();

        // Update the next execution
        let ticks_taken = time_slice.get() * task_info.tick_rate;
        task_info.next_execution.0 = task_info.next_execution.0.wrapping_add(ticks_taken);

        // Tell it it can run again!
        *running_state_guard = RunningState::CanRun {
            execution_cycles: time_slice,
        };
        drop(running_state_guard);

        // Unpark the thread
        task_info.thread.unpark();

        // Put it inflight
        inflight.push(task_info.task_state.clone());

        // Put it back on our heap and let it rearrange itself
        heap.push(task_info);
    }
}

fn wait_on_inflight(
    inflight: &mut Vec<Arc<TaskState>>,
    main_thread_queue: &MainThreadQueue,
    tick_real_time: Duration,
) {
    // Make sure all previous tasks have been stopped
    for inflight in inflight.drain(..) {
        loop {
            // Lock the state
            let running_state_guard = inflight.running_state.lock().unwrap();

            // Break if they are ready
            if *running_state_guard == RunningState::Waiting {
                break;
            }
            drop(running_state_guard);

            // Make sure anything that needs to run on main gets their turn
            main_thread_queue.main_thread_poll(tick_real_time);
        }
    }
}
