use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    num::NonZero,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
    vec::Vec,
};

use crossbeam::atomic::AtomicCell;
use itertools::Itertools;
use nohash::BuildNoHashHasher;
use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
pub use task::*;

use crate::{
    component::{Component, ErasedComponentHandle, ResourcePath},
    machine::registry::ComponentRegistry,
};

mod run;
mod task;
#[cfg(test)]
mod test;

#[derive(Debug, Clone)]
struct TaskMetadata {
    period: Ratio<u32>,
    path: ResourcePath,
}

#[derive(Debug)]
struct TimelineTaskEntry {
    pub task_id: TaskId,
    pub component: ErasedComponentHandle,
}

#[derive(Debug)]
struct TimelineEntry {
    pub tasks: Vec<TimelineTaskEntry>,
    pub time_slice: NonZero<u32>,
}

#[derive(Debug)]
struct Timeline {
    entries: BTreeMap<u32, TimelineEntry>,
    tick_real_time: Duration,
    length: u32,
}

/// The main scheduler that the runtime uses to drive tasks
///
/// It is a frequency based cooperative scheduler with some optional out of order execution stuff
#[derive(Debug)]
pub struct Scheduler {
    current_tick: u32,
    timeline: Option<Timeline>,
    registry: Arc<ComponentRegistry>,
    next_task_id: TaskId,
    task_metadata: HashMap<TaskId, TaskMetadata, BuildNoHashHasher<TaskId>>,
}

impl Scheduler {
    pub(crate) fn new(registry: Arc<ComponentRegistry>) -> Self {
        Self {
            current_tick: 0,
            timeline: None,
            task_metadata: HashMap::default(),
            registry,
            next_task_id: 0,
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn build_timeline(&mut self) {
        // empty schedule
        if self.task_metadata.is_empty() {
            return;
        }

        assert!(self.timeline.is_none(), "Timeline already computed!");

        let system_lcm = Ratio::new(
            self.task_metadata
                .values()
                .map(|task| *task.period.numer())
                .reduce(lcm)
                .unwrap_or(1),
            self.task_metadata
                .values()
                .map(|task| *task.period.denom())
                .reduce(gcd)
                .unwrap_or(1),
        );

        let system_gcd = Ratio::new(
            self.task_metadata
                .values()
                .map(|task| *task.period.numer())
                .reduce(gcd)
                .unwrap_or(1),
            self.task_metadata
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

        let mut timeline = BTreeMap::default();
        let mut current_tick = 0;

        while current_tick < timeline_length {
            let tasks: Vec<_> = self
                .task_metadata
                .iter()
                .map(|(task_id, task_metadata)| {
                    let relative_period = (task_metadata.period / system_gcd).to_u32().unwrap();

                    (task_id, task_metadata, current_tick % relative_period)
                })
                .sorted_by_key(|(_, _, time_till_run)| *time_till_run)
                .collect();

            let number_to_run = tasks
                .iter()
                .take_while(|(_, _, time_till_run)| *time_till_run == 0)
                .count();

            match number_to_run {
                0 => {
                    current_tick += 1;
                }
                1 => {
                    let (first_task_id, first_task_metadata, _) = tasks.first().unwrap();

                    let first_relative_period =
                        (first_task_metadata.period / system_gcd).to_u32().unwrap();

                    if let Some((_, second_task_metadata, _)) = tasks.get(1) {
                        // Calculate the time difference and schedule runs to fit within this difference

                        let second_relative_period =
                            (second_task_metadata.period / system_gcd).to_u32().unwrap();

                        let first_time_until = (first_relative_period
                            - (current_tick % first_relative_period))
                            % first_relative_period;
                        let second_time_until = (second_relative_period
                            - (current_tick % second_relative_period))
                            % second_relative_period;

                        let time_difference = second_time_until - first_time_until;
                        let runs_before_second = time_difference / first_relative_period;

                        let time_slice = runs_before_second.max(1);

                        timeline.insert(
                            current_tick,
                            TimelineEntry {
                                time_slice: NonZero::new(time_slice).unwrap(),
                                tasks: vec![TimelineTaskEntry {
                                    task_id: **first_task_id,
                                    component: self
                                        .registry
                                        .get_erased(&first_task_metadata.path.component)
                                        .unwrap(),
                                }],
                            },
                        );

                        current_tick += time_difference;
                    } else {
                        // Limit by itself

                        timeline.insert(
                            current_tick,
                            TimelineEntry {
                                time_slice: NonZero::new(1).unwrap(),
                                tasks: vec![TimelineTaskEntry {
                                    task_id: **first_task_id,
                                    component: self
                                        .registry
                                        .get_erased(&first_task_metadata.path.component)
                                        .unwrap(),
                                }],
                            },
                        );

                        current_tick += first_relative_period;
                    }
                }
                _ => {
                    timeline.insert(
                        current_tick,
                        TimelineEntry {
                            time_slice: NonZero::new(1).unwrap(),
                            tasks: tasks
                                .into_iter()
                                .map(|(task_id, _, _)| {
                                    let task_metadata = self.task_metadata.get(task_id).unwrap();

                                    TimelineTaskEntry {
                                        task_id: *task_id,
                                        component: self
                                            .registry
                                            .get_erased(&task_metadata.path.component)
                                            .unwrap(),
                                    }
                                })
                                .collect(),
                        },
                    );

                    current_tick += 1;
                }
            }
        }

        self.timeline = Some(Timeline {
            tick_real_time,
            entries: timeline,
            length: timeline_length,
        });
    }

    pub(crate) fn insert_task<C: Component>(
        &mut self,
        path: ResourcePath,
        period: Ratio<u32>,
        mut task: impl Task<C>,
    ) -> (TaskId, TaskData) {
        assert!(self.timeline.is_none(), "Timeline already computed!");

        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.checked_add(1).expect("Too many tasks");

        self.task_metadata
            .insert(task_id, TaskMetadata { period, path });

        let data = TaskData {
            callback: Box::new(move |component, slice| {
                // tracing::info!("Executing task for {} for {} units", path, slice);

                let component = (component as &mut dyn Any).downcast_mut().unwrap();

                task.run(component, slice);
            }),
        };

        (task_id, data)
    }

    /// Move the scheduler onto a dedicated thread, where it will be automatically executed and managed
    ///
    /// TODO: Add a handle so the runtime can tear down the thread
    /// TODO: Make the dedicated thread implementation actually efficient
    pub fn spawn_dedicated_thread(self) {
        /*
        std::thread::spawn(|| {
            scheduler_thread(self);
        });
         */
    }

    /// Get the length of the timeline, in ticks
    pub fn timeline_length(&self) -> u32 {
        self.timeline
            .as_ref()
            .expect("Timeline has yet to be initialized")
            .length
    }
}

/// Handle to control the scheduler while it is on a dedicated thread
#[derive(Debug)]
pub struct SchedulerHandle {
    average_efficiency: AtomicCell<f32>,
    paused: AtomicBool,
    exit: AtomicBool,
}

impl SchedulerHandle {
    /// Pause execution
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Release);
    }

    /// Continue execution
    pub fn play(&self) {
        self.paused.store(false, Ordering::Release);
    }
}
