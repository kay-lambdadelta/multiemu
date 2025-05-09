use crate::component::{Component, ComponentId, store::ComponentStore};
use itertools::Itertools;
use num::{ToPrimitive, integer::lcm, rational::Ratio};
use rustc_hash::FxBuildHasher;
use std::{
    collections::HashMap,
    fmt::Debug,
    num::NonZero,
    sync::{Arc, Mutex},
    time::Duration,
};

type TaskId = u16;

pub mod task;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToRun {
    pub task_id: TaskId,
    pub run_indication: u32,
    pub tick_rate: u32,
}

struct StoredTask {
    pub component_id: ComponentId,
    #[allow(clippy::type_complexity)]
    pub task: Mutex<Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>>,
    pub frequency: Ratio<u32>,
}

impl Debug for StoredTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredTask")
            .field("component_id", &self.component_id)
            .field("frequency", &self.frequency)
            .finish()
    }
}

#[derive(Debug)]
pub struct Scheduler {
    current_tick: u32,
    common_denominator: u32,
    tick_real_time: Duration,
    allotted_time: Duration,
    component_store: Arc<ComponentStore>,
    tasks: HashMap<TaskId, StoredTask, FxBuildHasher>,
}

impl Scheduler {
    pub fn new(
        component_store: Arc<ComponentStore>,
        tasks: impl IntoIterator<
            Item = (
                ComponentId,
                Ratio<u32>,
                Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>,
            ),
        >,
    ) -> Self {
        let tasks: HashMap<_, _, FxBuildHasher> = tasks
            .into_iter()
            .enumerate()
            .map(|(task_id, (component_id, frequency, task))| {
                let task_id = task_id.try_into().expect("Too many tasks");

                tracing::debug!(
                    "Task {} needs to run every {:?}",
                    task_id,
                    Duration::from_secs_f64(frequency.recip().to_f64().unwrap()),
                );

                (
                    task_id,
                    StoredTask {
                        task: Mutex::new(task),
                        component_id,
                        frequency: frequency.recip(),
                    },
                )
            })
            .collect();

        let common_denominator = tasks
            .values()
            .map(|task| *task.frequency.denom())
            .fold(1, lcm);

        let common_numerator = tasks
            .values()
            .map(|task| *task.frequency.numer())
            .fold(1, lcm);

        let full_cycle = common_denominator / common_numerator;
        let tick_real_time = Duration::from_secs_f64(
            Ratio::new(common_numerator, common_denominator)
                .to_f64()
                .unwrap(),
        );

        tracing::debug!(
            "Schedule ticks take {:?} and restarts at tick {}, a full cycle takes {:?}",
            tick_real_time,
            common_denominator,
            tick_real_time * full_cycle as u32
        );

        Self {
            current_tick: 0,
            common_denominator,
            tick_real_time,
            allotted_time: Duration::from_secs_f64(Ratio::new(1, 60).to_f64().unwrap()),
            component_store,
            tasks: tasks.into_iter().collect(),
        }
    }

    pub fn run(&mut self) {
        // TODO: This should actually be calculating how much time is between frames minus draw time
        let alloted_ticks_this_pass: u32 = (self.allotted_time.as_nanos()
            / self.tick_real_time.as_nanos())
        .try_into()
        .unwrap();
        let old_current_tick = self.current_tick;

        while (self.current_tick.wrapping_sub(old_current_tick)) < alloted_ticks_this_pass {
            let to_run: Vec<_> = self
                .tasks
                .iter()
                .map(|(task_id, stored_task)| {
                    let factor = self.common_denominator / stored_task.frequency.denom();
                    let tick_rate = stored_task.frequency.numer() * factor;

                    ToRun {
                        task_id: *task_id,
                        run_indication: self.current_tick % tick_rate,
                        tick_rate,
                    }
                })
                .sorted_by_key(|to_run| to_run.run_indication)
                .collect();

            if to_run.len() == 1 {
                let to_run_info = to_run[0];
                let time_slice = to_run_info.tick_rate;
                self.run_task([(to_run_info.task_id, NonZero::new(time_slice).unwrap())]);

                let time_slice_occupying_time = time_slice * to_run_info.tick_rate;
                self.current_tick = self
                    .current_tick
                    .checked_add(time_slice_occupying_time)
                    .unwrap()
                    % self.common_denominator;

                continue;
            }

            // do the different scenarios for how many should run this turn
            match to_run
                .iter()
                .filter(|to_run| to_run.run_indication == 0)
                .count()
            {
                // Nothing is set to run here
                0 => {
                    self.current_tick =
                        self.current_tick.checked_add(1).unwrap() % self.common_denominator;
                }
                // Full efficient batching
                1 => {
                    let batch_size = to_run[1].tick_rate - to_run[1].run_indication;
                    let to_run_info = to_run[0];
                    let time_slice = batch_size / to_run_info.tick_rate;

                    if let Some(time_slice) = NonZero::new(time_slice) {
                        self.run_task([(to_run_info.task_id, time_slice)]);

                        let time_slice_occupying_time = time_slice.get() * to_run_info.tick_rate;
                        self.current_tick = self
                            .current_tick
                            .checked_add(time_slice_occupying_time)
                            .unwrap()
                            % self.common_denominator;
                    } else {
                        self.current_tick =
                            self.current_tick.checked_add(1).unwrap() % self.common_denominator;
                    }
                }
                // Conflicted components
                _ => {
                    self.run_task(to_run.into_iter().filter_map(|to_run_info| {
                        if to_run_info.run_indication == 0 {
                            return Some((to_run_info.task_id, NonZero::new(1).unwrap()));
                        }

                        None
                    }));

                    self.current_tick =
                        self.current_tick.checked_add(1).unwrap() % self.common_denominator;
                }
            }
        }
    }

    #[inline]
    fn run_task(&mut self, to_run: impl IntoIterator<Item = (TaskId, NonZero<u32>)>) {
        for (task_id, time_slice) in to_run {
            let task_info = self.tasks.get(&task_id).unwrap();
            self.component_store
                .interact_dyn(task_info.component_id, |component| {
                    let mut task = task_info.task.lock().unwrap();
                    task(component, time_slice);
                })
                .unwrap();
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
            .min(self.tick_real_time * self.common_denominator);

        tracing::trace!(
            "Alotted time for scheduler moved up to {:?}",
            self.allotted_time
        );
    }
}
