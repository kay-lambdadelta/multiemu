use crate::{builder::task::StoredTask, component::ComponentId};
use nohash::BuildNoHashHasher;
use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
use std::{
    boxed::Box,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    num::NonZero,
    time::Duration,
    vec::Vec,
};
pub use task::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionMode {
    Single,
    Multi,
}

mod run;
mod task;

/// TODO: Future me, make this deterministically multithreaded

pub type TaskId = u16;

struct TaskInfo {
    pub task: Box<dyn Task>,
    pub relative_period: u32,
}

impl Debug for TaskInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskInfo")
            .field("tick_rate", &self.relative_period)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScheduleEntry {
    pub task_id: TaskId,
    pub time_slice: NonZero<u32>,
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
    /// The amount of time a tick takes
    tick_real_time: Duration,
    /// Active tasks
    timeline: Vec<Vec<ScheduleEntry>>,
    tasks: Vec<TaskInfo>,
    /// Indexed by component id, slight speed boost over nohash
    component_tasks: HashMap<ComponentId, Vec<TaskId>, BuildNoHashHasher<u16>>,
}

impl Scheduler {
    pub(crate) fn new(
        component_tasks: HashMap<ComponentId, HashMap<TaskName, StoredTask>>,
    ) -> Self {
        // Only the active tasks are put on the schedule
        let mut tasks: BTreeMap<u16, _> = BTreeMap::new();
        let mut component_owned_tasks: HashMap<_, Vec<_>, _> = HashMap::default();

        for (component_id, task_id, _task_name, task) in component_tasks
            .into_iter()
            .flat_map(|(component_id, tasks)| {
                tasks.into_iter().map(move |task| (component_id, task))
            })
            .enumerate()
            .map(|(task_id, (component_id, (task_name, task)))| {
                (component_id, task_id.try_into().unwrap(), task_name, task)
            })
        {
            tasks.insert(task_id, task);

            component_owned_tasks
                .entry(component_id)
                .or_default()
                .push(task_id);
        }

        let system_lcm = Ratio::new(
            tasks
                .values()
                .map(|task| *task.period.numer())
                .reduce(lcm)
                .unwrap_or(1),
            tasks
                .values()
                .map(|task| *task.period.denom())
                .reduce(gcd)
                .unwrap_or(1),
        );

        let system_gcd = Ratio::new(
            tasks
                .values()
                .map(|task| *task.period.numer())
                .reduce(gcd)
                .unwrap_or(1),
            tasks
                .values()
                .map(|task| *task.period.denom())
                .reduce(lcm)
                .unwrap_or(1),
        );

        let timeline_length = system_lcm / system_gcd;
        assert!(timeline_length.is_integer());
        let timeline_length = timeline_length.to_integer();

        let tick_real_time = Duration::from_secs_f64(system_gcd.to_f64().unwrap());
        tracing::info!(
            "Schedule ticks take {:?} ({} of a second) and the full timeline takes {:?} ({} ticks)",
            tick_real_time,
            system_gcd,
            timeline_length * tick_real_time,
            timeline_length,
        );

        let mut timeline = vec![Vec::new(); timeline_length as usize];

        let tasks: Vec<_> = tasks
            .into_iter()
            .map(|(task_id, task)| {
                let relative_period = task.period / system_gcd;

                tracing::debug!(
                    "Task {} has a period of {} ({:?}), tick rate of {}",
                    task_id,
                    task.period,
                    Duration::from_secs_f64(task.period.to_f64().unwrap()),
                    relative_period
                );

                assert!(relative_period.is_integer());
                assert!(timeline_length % relative_period.to_integer() == 0);

                TaskInfo {
                    task: task.task,
                    relative_period: relative_period.to_integer(),
                }
            })
            .collect();

        let mut schedule = BTreeMap::from_iter([(0, (0..tasks.len() as u16).collect::<Vec<_>>())]);

        for (current_tick, timeline_tick_entries) in timeline.iter_mut().enumerate() {
            let current_tick = current_tick as u32;

            // It's OK to use [Option::unwrap_or_default] here; an empty Vec does not allocate in Rust
            let active_events = schedule.remove(&current_tick).unwrap_or_default();

            let active_max_allotted_ticks =
                timeline_length.min(timeline_length - current_tick).min(
                    schedule
                        .range(current_tick..)
                        .next()
                        .map(|(tick, _)| *tick - current_tick)
                        .unwrap_or(timeline_length),
                );

            match active_events.len() {
                0 => {}
                1 => {
                    let task_id = active_events[0];
                    let task_info = &tasks[task_id as usize];
                    let time_slice: NonZero<u32> = NonZero::new(
                        (active_max_allotted_ticks / task_info.relative_period).max(1),
                    )
                    .unwrap();
                    let representing_time = time_slice.get() * task_info.relative_period;

                    schedule
                        .entry((current_tick + representing_time) % timeline_length)
                        .or_default()
                        .push(task_id);

                    timeline_tick_entries.push(ScheduleEntry {
                        task_id,
                        time_slice,
                    });
                }
                _ => {
                    for task_id in active_events {
                        let task_info = &tasks[task_id as usize];
                        let time_slice = NonZero::new(1).unwrap();
                        let representing_time = time_slice.get() * task_info.relative_period;

                        schedule
                            .entry((current_tick + representing_time) % timeline_length)
                            .or_default()
                            .push(task_id);

                        timeline_tick_entries.push(ScheduleEntry {
                            task_id,
                            time_slice,
                        });
                    }
                }
            }
        }

        Self {
            current_tick: 0,
            tick_real_time,
            tasks,
            component_tasks: component_owned_tasks,
            timeline,
        }
    }

    #[inline]
    fn update_current_tick(&mut self, amount: u32) {
        self.current_tick =
            self.current_tick.checked_add(amount).unwrap() % self.timeline.len() as u32;
    }

    pub fn full_cycle(&self) -> u32 {
        self.timeline.len() as u32
    }
}
