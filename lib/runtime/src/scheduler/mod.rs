use crate::{
    component::{Component, ErasedComponentHandle, ResourcePath},
    machine::registry::ComponentRegistry,
    scheduler::run::scheduler_thread,
};
use crossbeam::atomic::AtomicCell;
use itertools::Itertools;
use nohash::BuildNoHashHasher;
use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
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

pub use task::*;

mod run;
mod task;
#[cfg(test)]
mod test;

#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub id: TaskId,
    pub period: Ratio<u32>,
    pub path: ResourcePath,
    pub ty: TaskType,
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
pub struct Timeline {
    entries: BTreeMap<u32, TimelineEntry>,
    tick_real_time: Duration,
    timeline_length: u32,
}

/// The scheduler for the emulator
///
/// It is a cooperative multithreaded tick based scheduler
///
/// Currently it only supports frequency based executions
#[derive(Debug)]
pub struct Scheduler {
    current_tick: u32,
    timeline: Option<Timeline>,
    registry: Arc<ComponentRegistry>,
    handle: Arc<SchedulerHandle>,
    next_task_id: TaskId,
    task_metadata: HashMap<TaskId, TaskMetadata, BuildNoHashHasher<TaskId>>,
}

impl Scheduler {
    pub(crate) fn new(registry: Arc<ComponentRegistry>) -> Self {
        let handle = SchedulerHandle {
            average_efficiency: AtomicCell::new(1.0),
            paused: AtomicBool::new(true),
            exit: AtomicBool::new(false),
        };

        Self {
            current_tick: 0,
            timeline: None,
            task_metadata: HashMap::default(),
            registry,
            handle: Arc::new(handle),
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

                    if let Some((_, second_task_metadata, _)) = tasks
                        .iter()
                        .skip(1)
                        .find(|(_, task_metadata, _)| task_metadata.ty == TaskType::Direct)
                    {
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
            timeline_length,
        });
    }

    pub(crate) fn insert_task<C: Component>(
        &mut self,
        path: ResourcePath,
        ty: TaskType,
        period: Ratio<u32>,
        mut task: impl Task<C>,
    ) -> (TaskId, TaskData) {
        assert!(self.timeline.is_none(), "Timeline already computed!");

        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.checked_add(1).expect("Too many tasks");

        self.task_metadata.insert(
            task_id,
            TaskMetadata {
                id: task_id,
                period,
                path: path.clone(),
                ty,
            },
        );

        let data = TaskData {
            callback: Box::new(move |component, slice| {
                // tracing::info!("Executing task for {} for {} units", path, slice);

                let component = (component as &mut dyn Any).downcast_mut().unwrap();

                task.run(component, slice)
            }),
            debt: 0,
            ty,
        };

        (task_id, data)
    }

    /// Move the scheduler onto a dedicated thread
    pub fn spawn_dedicated_thread(self) {
        std::thread::spawn(|| {
            scheduler_thread(self);
        });
    }

    pub fn handle(&self) -> Arc<SchedulerHandle> {
        self.handle.clone()
    }

    pub fn timeline_length(&self) -> u32 {
        self.timeline.as_ref().unwrap().timeline_length
    }
}

#[derive(Debug)]
pub struct SchedulerHandle {
    average_efficiency: AtomicCell<f32>,
    paused: AtomicBool,
    exit: AtomicBool,
}

impl SchedulerHandle {
    /// Pause execution
    ///
    /// This does nothing when not using a dedicated thread for the scheduler
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Release);
    }

    /// Play execution
    ///
    /// This does nothing when not using a dedicated thread for the scheduler
    pub fn play(&self) {
        self.paused.store(false, Ordering::Release);
    }

    /// Get the average efficiency of the scheduler
    pub fn average_efficiency(&self) -> f32 {
        self.average_efficiency.load()
    }
}
