use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
use rustc_hash::FxBuildHasher;
use std::{
    boxed::Box,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    num::NonZero,
    sync::Mutex,
    time::Duration,
    vec::Vec,
};
pub use task::*;

mod run;
mod task;

/// TODO: Future me, make this deterministically multithreaded

pub type TaskId = u16;

struct TaskInfo {
    pub task: Mutex<Box<dyn Task>>,
    pub tick_rate: u32,
}

impl Debug for TaskInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskInfo")
            .field("tick_rate", &self.tick_rate)
            .finish()
    }
}

#[derive(Debug)]
struct TaskToExecute {
    pub time_slice: NonZero<u32>,
    pub id: TaskId,
}

/// The scheduler for the emulator
///
/// It is a cooperative multithreaded tick based scheduler
///
/// Currently it only supports frequency based executions'
#[derive(Debug)]
pub struct Scheduler {
    /// Global tick we are currently on
    current_tick: u32,
    /// Rollover tick
    ticks_per_full_cycle: u32,
    /// The amount of time a tick takes
    tick_real_time: Duration,
    /// Tasks
    tasks: HashMap<TaskId, TaskInfo, FxBuildHasher>,
    schedule: BTreeMap<u32, Vec<TaskId>>,
}

impl Scheduler {
    pub fn allotted_time(&self) -> Duration {
        self.tick_real_time * self.ticks_per_full_cycle
    }

    pub(crate) fn new(tasks: impl IntoIterator<Item = (Ratio<u32>, Box<dyn Task>)>) -> Self {
        pub struct PrecalculationTask {
            #[allow(clippy::type_complexity)]
            pub task: Box<dyn Task>,
            pub frequency: Ratio<u32>,
        }

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

        tracing::debug!("System frequency is {}", common);

        let tick_real_time = Duration::from_secs_f64(common.to_f64().unwrap());
        let ticks_per_full_cycle = common.recip().to_integer();

        tracing::debug!(
            "Schedule ticks take {:?} and rolls over at tick {}, a full cycle takes {:?}",
            tick_real_time,
            ticks_per_full_cycle,
            tick_real_time * ticks_per_full_cycle as u32
        );

        let tasks: HashMap<_, _, _> = tasks
            .into_iter()
            .enumerate()
            .map(|(task_id, precalcuation_task)| {
                let tick_rate = (Ratio::from_integer(ticks_per_full_cycle)
                    / precalcuation_task.frequency.recip())
                .to_integer();

                let task_id = task_id.try_into().unwrap();

                (
                    task_id,
                    TaskInfo {
                        task: Mutex::new(precalcuation_task.task),
                        tick_rate,
                    },
                )
            })
            .collect();

        let schedule = BTreeMap::from_iter([(0, tasks.keys().copied().collect())]);

        Self {
            current_tick: 0,
            tick_real_time,
            ticks_per_full_cycle,
            tasks,
            schedule,
        }
    }

    #[inline]
    fn run_tasks(&mut self, timeline: impl IntoIterator<Item = TaskToExecute>) {
        for TaskToExecute { id, time_slice } in timeline {
            let representing_time = time_slice.get() * self.tasks.get(&id).unwrap().tick_rate;

            self.schedule
                .entry((self.current_tick + representing_time) % self.ticks_per_full_cycle)
                .or_default()
                .push(id);

            let mut task = self.tasks.get(&id).unwrap().task.lock().unwrap();

            task.run(time_slice);
        }

        self.update_current_tick(1);
    }

    #[inline]
    fn update_current_tick(&mut self, amount: u32) {
        self.current_tick =
            self.current_tick.checked_add(amount).unwrap() % self.ticks_per_full_cycle;
    }
}
