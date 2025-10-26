use crate::{
    component::{Component, ErasedComponentHandle, ResourcePath},
    machine::registry::ComponentRegistry,
    scheduler::run::scheduler_thread,
};
use crossbeam::atomic::AtomicCell;
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
pub struct ScheduleEntry {
    pub task_id: TaskId,
    pub time_slice: NonZero<u32>,
    pub component: ErasedComponentHandle,
}

#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub id: TaskId,
    pub period: Ratio<u32>,
    pub path: ResourcePath,
}

#[derive(Debug)]
pub struct Timeline {
    entries: Vec<Vec<ScheduleEntry>>,
    tick_real_time: Duration,
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
    tasks: HashMap<TaskId, TaskMetadata, BuildNoHashHasher<TaskId>>,
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
            tasks: HashMap::default(),
            registry,
            handle: Arc::new(handle),
            next_task_id: 0,
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn build_timeline(&mut self) {
        assert!(self.timeline.is_none(), "Timeline already computed!");

        let system_lcm = Ratio::new(
            self.tasks
                .values()
                .map(|task| *task.period.numer())
                .reduce(lcm)
                .unwrap_or(1),
            self.tasks
                .values()
                .map(|task| *task.period.denom())
                .reduce(gcd)
                .unwrap_or(1),
        );

        let system_gcd = Ratio::new(
            self.tasks
                .values()
                .map(|task| *task.period.numer())
                .reduce(gcd)
                .unwrap_or(1),
            self.tasks
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

        let mut schedule =
            BTreeMap::from_iter([(0, self.tasks.keys().cloned().collect::<Vec<_>>())]);

        for (current_tick, timeline_tick_entries) in timeline.iter_mut().enumerate() {
            let current_tick = current_tick as u32;

            // It's OK to use [Option::unwrap_or_default] here; an empty Vec does not allocate in Rust
            let mut active_events = schedule.remove(&current_tick).unwrap_or_default();

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
                    let task_path = active_events.remove(0);
                    let task_metadata = &self.tasks[&task_path];
                    let relative_period = (task_metadata.period / system_gcd).to_u32().unwrap();

                    let time_slice: NonZero<u32> =
                        NonZero::new((active_max_allotted_ticks / relative_period).max(1)).unwrap();
                    let representing_time = time_slice.get() * relative_period;

                    let component = self
                        .registry
                        .get_erased(&task_metadata.path.component)
                        .unwrap();
                    schedule
                        .entry((current_tick + representing_time) % timeline_length)
                        .or_default()
                        .push(task_path);

                    timeline_tick_entries.push(ScheduleEntry {
                        task_id: task_metadata.id,
                        time_slice,
                        component,
                    });
                }
                _ => {
                    for task_path in active_events {
                        let task_metadata = &self.tasks[&task_path];
                        let relative_period = (task_metadata.period / system_gcd).to_u32().unwrap();
                        let time_slice = NonZero::new(1).unwrap();

                        let representing_time = time_slice.get() * relative_period;

                        let component = self
                            .registry
                            .get_erased(&task_metadata.path.component)
                            .unwrap();

                        schedule
                            .entry((current_tick + representing_time) % timeline_length)
                            .or_default()
                            .push(task_path);

                        timeline_tick_entries.push(ScheduleEntry {
                            task_id: task_metadata.id,
                            time_slice,
                            component,
                        });
                    }
                }
            }
        }

        self.timeline = Some(Timeline {
            tick_real_time,
            entries: timeline,
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

        self.tasks.insert(
            task_id,
            TaskMetadata {
                id: task_id,
                period,
                path: path.clone(),
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
        self.timeline.as_ref().unwrap().entries.len() as u32
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
