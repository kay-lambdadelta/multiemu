use crate::component::{store::ComponentStore, Component, ComponentId};
use itertools::Itertools;
use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
use std::{
    fmt::Debug,
    num::NonZero,
    sync::{Arc, Mutex},
    time::Duration,
};

pub mod task;

#[derive(Debug, Clone, Copy)]
struct ToRun<'a> {
    pub run_indication: u32,
    pub task_info: &'a StoredTask,
}

struct StoredTask {
    pub component_id: ComponentId,
    #[allow(clippy::type_complexity)]
    pub task: Mutex<Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>>,
    pub frequency: Ratio<u32>,
    pub relative_tick_rate: u32,
}

impl Debug for StoredTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredTask")
            .field("component_id", &self.component_id)
            .field("frequency", &self.frequency)
            .field("relative_tick_rate", &self.relative_tick_rate)
            .finish()
    }
}

#[derive(Debug)]
pub struct Scheduler {
    current_tick: u32,
    rollover_tick: u32,
    tick_real_time: Duration,
    allotted_time: Duration,
    component_store: Arc<ComponentStore>,
    tasks: Vec<StoredTask>,
}

impl Scheduler {
    pub(crate) fn new(
        component_store: Arc<ComponentStore>,
        tasks: impl IntoIterator<
            Item = (
                ComponentId,
                Ratio<u32>,
                Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>,
            ),
        >,
    ) -> Self {
        let tasks: Vec<_> = tasks
            .into_iter()
            .enumerate()
            .map(|(task_id, (component_id, frequency, task))| {
                tracing::debug!(
                    "Task {} has a frequency of {} (period of {:?})",
                    task_id,
                    frequency,
                    Duration::from_secs_f64(frequency.recip().to_f64().unwrap())
                );

                StoredTask {
                    task: Mutex::new(task),
                    component_id,
                    frequency: frequency.recip(),
                    relative_tick_rate: 0,
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

        tracing::info!("System frequency is {}", common);

        let tick_real_time = Duration::from_secs_f64(common.to_f64().unwrap());
        let rollover_tick = common.recip().to_integer();

        tracing::debug!(
            "Schedule ticks take {:?} and rolls over at tick {}, a full cycle takes {:?}",
            tick_real_time,
            rollover_tick,
            tick_real_time * rollover_tick as u32
        );

        Self {
            current_tick: 0,
            rollover_tick,
            tick_real_time,
            allotted_time: Duration::from_secs_f64(Ratio::new(1, 60).to_f64().unwrap()),
            component_store,
            tasks: tasks
                .into_iter()
                .map(|mut stored_task| {
                    let factor = rollover_tick / stored_task.frequency.denom();
                    let tick_rate = stored_task.frequency.numer() * factor;

                    stored_task.relative_tick_rate = tick_rate;

                    stored_task
                })
                .sorted_unstable_by_key(|stored_task| stored_task.relative_tick_rate)
                .collect(),
        }
    }

    pub fn run(&mut self) {
        let alloted_ticks_this_pass: u32 = (self.allotted_time.as_nanos()
            / self.tick_real_time.as_nanos())
        .try_into()
        .unwrap();
        let old_current_tick = self.current_tick;

        while (self.current_tick.wrapping_sub(old_current_tick)) < alloted_ticks_this_pass {
            let to_run = self.tasks.iter().map(|task| ToRun {
                run_indication: self.current_tick % task.relative_tick_rate,
                task_info: task,
            });

            let mut to_run_now = to_run.clone().filter(|to_run| to_run.run_indication == 0);

            if to_run.len() == 1 {
                let to_run_now = to_run_now.next().unwrap();
                let time_slice = to_run_now.task_info.relative_tick_rate;
                self.run_task([(to_run_now, NonZero::new(time_slice).unwrap())]);

                let time_slice_occupying_time =
                    time_slice * to_run_now.task_info.relative_tick_rate;
                self.current_tick = self
                    .current_tick
                    .checked_add(time_slice_occupying_time)
                    .unwrap()
                    % self.rollover_tick;

                continue;
            }

            // do the different scenarios for how many should run this turn
            match to_run_now.clone().count() {
                // Nothing is set to run here
                0 => {
                    self.current_tick =
                        self.current_tick.checked_add(1).unwrap() % self.rollover_tick;
                }
                // Full efficient batching
                1 => {
                    let to_run_next = to_run
                        .filter(|to_run| to_run.run_indication != 0)
                        .min_by_key(|to_run| to_run.run_indication)
                        .unwrap();
                    let to_run_now = to_run_now.next().unwrap();

                    let batch_size =
                        to_run_next.task_info.relative_tick_rate - to_run_next.run_indication;
                    let time_slice = batch_size / to_run_now.task_info.relative_tick_rate;

                    if let Some(time_slice) = NonZero::new(time_slice) {
                        self.run_task([(to_run_now, time_slice)]);

                        let time_slice_occupying_time =
                            time_slice.get() * to_run_now.task_info.relative_tick_rate;
                        self.current_tick = self
                            .current_tick
                            .checked_add(time_slice_occupying_time)
                            .unwrap()
                            % self.rollover_tick;
                    } else {
                        self.current_tick =
                            self.current_tick.checked_add(1).unwrap() % self.rollover_tick;
                    }
                }
                // Conflicted components
                _ => {
                    self.run_task(
                        to_run_now.map(|to_run_now| (to_run_now, NonZero::new(1).unwrap())),
                    );

                    self.current_tick =
                        self.current_tick.checked_add(1).unwrap() % self.rollover_tick;
                }
            }
        }
    }

    #[inline]
    fn run_task<'a>(&self, to_run: impl IntoIterator<Item = (ToRun<'a>, NonZero<u32>)>) {
        for (to_run, time_slice) in to_run {
            self.component_store
                .interact_dyn(to_run.task_info.component_id, |component| {
                    let mut task = to_run.task_info.task.lock().unwrap();

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
            .min(self.tick_real_time * self.rollover_tick);

        tracing::trace!(
            "Alotted time for scheduler moved up to {:?}",
            self.allotted_time
        );
    }
}
