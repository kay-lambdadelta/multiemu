use nohash::BuildNoHashHasher;
use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
use serde::{Deserialize, Serialize};
use std::{
    boxed::Box,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    num::NonZero,
    sync::{Arc, Mutex},
    time::Duration,
    vec::Vec,
};
pub use task::*;

use crate::{builder::task::StoredTask, component::ComponentId};

mod run;
mod task;

/// TODO: Future me, make this deterministically multithreaded

pub type TaskId = u16;

#[derive(Debug)]
enum TaskMode {
    Active,
    Lazy { debt: u32 },
}

struct TaskInfo {
    pub task: Box<dyn Task>,
    pub tick_rate: u32,
    pub mode: TaskMode,
}

impl Debug for TaskInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskInfo")
            .field("tick_rate", &self.tick_rate)
            .field("mode", &self.mode)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DebtClearer(Arc<TaskStorage>);

impl DebtClearer {
    #[inline]
    /// Make sure this component is up to date
    pub fn clear_debts(&self, component_id: ComponentId) {
        for task_id in self
            .0
            .component_tasks
            .get(&component_id)
            .into_iter()
            .flatten()
        {
            let Ok(mut task_info) = self.0.tasks.get(task_id).unwrap().try_lock() else {
                continue;
            };

            if let TaskMode::Lazy { debt } = &mut task_info.mode {
                let old_debt = *debt;
                *debt = 0;

                if let Some(debt) = NonZero::new(old_debt) {
                    task_info.task.run(debt);
                }
            }
        }
    }
}

#[derive(Debug)]
struct TaskStorage {
    tasks: HashMap<TaskId, Mutex<TaskInfo>, BuildNoHashHasher<u16>>,
    component_tasks: HashMap<ComponentId, Vec<TaskId>, BuildNoHashHasher<u16>>,
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
    /// Active tasks
    active_schedule: BTreeMap<u32, Vec<TaskId>>,
    /// Lazy tasks
    lazy_schedule: BTreeMap<u32, Vec<TaskId>>,
    /// Tasks
    storage: Arc<TaskStorage>,
}

#[derive(Serialize, Deserialize)]
struct SchedulerState {
    current_tick: u32,
    ticks_per_full_cycle: u32,
    tick_real_time: Duration,
    active_schedule: BTreeMap<u32, Vec<TaskId>>,
    lazy_schedule: BTreeMap<u32, Vec<TaskId>>,
}

impl Scheduler {
    pub fn allotted_time(&self) -> Duration {
        self.tick_real_time * self.ticks_per_full_cycle
    }

    pub(crate) fn new(component_tasks: HashMap<ComponentId, Vec<StoredTask>>) -> Self {
        // Only the active tasks are put on the schedule
        let mut tasks: BTreeMap<u16, _> = BTreeMap::new();
        let mut component_owned_tasks: HashMap<_, Vec<_>, _> = HashMap::default();

        for (component_id, task_id, task) in component_tasks
            .into_iter()
            .flat_map(|(component_id, tasks)| {
                tasks.into_iter().map(move |task| (component_id, task))
            })
            .enumerate()
            .map(|(task_id, (component_id, task))| {
                (component_id, task_id.try_into().unwrap(), task)
            })
        {
            tasks.insert(task_id, task);

            component_owned_tasks
                .entry(component_id)
                .or_default()
                .push(task_id);
        }

        let common = Ratio::new(
            tasks
                .values()
                .map(|task| *task.frequency.numer())
                .fold(1, gcd),
            tasks
                .values()
                .map(|task| *task.frequency.denom())
                .fold(1, lcm),
        );

        tracing::debug!("System frequency is {}", common);

        let tick_real_time = Duration::from_secs_f64(common.to_f64().unwrap());
        let ticks_per_full_cycle = common.recip().to_integer();

        tracing::info!(
            "Schedule ticks take {:?} and rolls over at tick {}, a full cycle takes {:?}",
            tick_real_time,
            ticks_per_full_cycle,
            tick_real_time * ticks_per_full_cycle as u32
        );

        let active_schedule = BTreeMap::from_iter([(
            0,
            tasks
                .iter()
                .filter_map(
                    |(task_id, task)| {
                        if task.lazy { None } else { Some(*task_id) }
                    },
                )
                .collect(),
        )]);

        let lazy_schedule = BTreeMap::from_iter([(
            0,
            tasks
                .iter()
                .filter_map(
                    |(task_id, task)| {
                        if !task.lazy { None } else { Some(*task_id) }
                    },
                )
                .collect(),
        )]);

        let tasks = tasks
            .into_iter()
            .map(|(task_id, task)| {
                let tick_rate = (Ratio::from_integer(ticks_per_full_cycle)
                    / task.frequency.recip())
                .to_integer();

                (
                    task_id,
                    Mutex::new(TaskInfo {
                        mode: if task.lazy {
                            TaskMode::Lazy { debt: 0 }
                        } else {
                            TaskMode::Active
                        },
                        task: task.task,
                        tick_rate,
                    }),
                )
            })
            .collect();

        Self {
            current_tick: 0,
            tick_real_time,
            ticks_per_full_cycle,
            storage: Arc::new(TaskStorage {
                tasks,
                component_tasks: component_owned_tasks,
            }),
            active_schedule,
            lazy_schedule,
        }
    }

    #[inline]
    fn update_current_tick(&mut self, amount: u32) {
        self.current_tick =
            self.current_tick.checked_add(amount).unwrap() % self.ticks_per_full_cycle;
    }

    pub(crate) fn get_debt_clearer(&self) -> DebtClearer {
        DebtClearer(self.storage.clone())
    }
}
