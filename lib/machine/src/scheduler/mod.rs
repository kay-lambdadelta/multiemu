use crate::component::store::ComponentStore;
use crate::component::{Component, ComponentId};
use fxhash::FxBuildHasher;
use itertools::Itertools;
use num::ToPrimitive;
use num::{Integer, integer::lcm, rational::Ratio};
use rangemap::RangeInclusiveMap;
use std::sync::{Arc, Mutex};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

type TaskId = u16;

pub mod task;

struct StoredTask {
    pub component_id: ComponentId,
    pub task: Mutex<Box<dyn FnMut(&dyn Component, u64) + Send + 'static>>,
}

pub struct Scheduler {
    current_tick: u64,
    rollover_tick: u64,
    tick_real_time: Ratio<u64>,
    allotted_time: Duration,
    component_store: Arc<ComponentStore>,
    schedule: RangeInclusiveMap<u64, Vec<TaskId>>,
    tasks: HashMap<TaskId, StoredTask, FxBuildHasher>,
}

impl Scheduler {
    pub fn new(
        component_store: Arc<ComponentStore>,
        tasks: impl IntoIterator<
            Item = (
                ComponentId,
                Ratio<u64>,
                Box<dyn FnMut(&dyn Component, u64) + Send + 'static>,
            ),
        >,
    ) -> Self {
        let tasks: HashMap<_, _, FxBuildHasher> = tasks
            .into_iter()
            .enumerate()
            .map(|(task_id, (component_id, frequency, task))| {
                (
                    task_id.try_into().expect("Too many tasks"),
                    (
                        StoredTask {
                            task: Mutex::new(task),
                            component_id,
                        },
                        frequency,
                    ),
                )
            })
            .collect();

        let common_denominator = tasks
            .values()
            .map(|(_, frequency)| *frequency.recip().denom())
            .fold(1, |acc, denom| acc.lcm(&denom));

        // Adjust numerators to the common denominator
        let adjusted_numerators: HashMap<_, _, FxBuildHasher> = tasks
            .iter()
            .map(|(task_id, (_, frequency))| {
                let factor = common_denominator / frequency.denom();
                (*task_id, frequency.numer() * factor)
            })
            .collect();

        let common_multiple = adjusted_numerators
            .clone()
            .into_values()
            .reduce(lcm)
            .unwrap_or(1);

        let ratios: HashMap<_, _, FxBuildHasher> = adjusted_numerators
            .iter()
            .map(|(task_id, numerator)| (*task_id, common_multiple / numerator))
            .collect();

        // Fill out the schedule
        let mut schedule = RangeInclusiveMap::default();

        let mut current_tick = 0;
        while current_tick < common_denominator {
            // This is (task_id, tick_rate, run_indication)
            let to_run: Vec<_> = ratios
                .iter()
                .map(|(task_id, tick_rate)| (*task_id, current_tick % *tick_rate, *tick_rate))
                .sorted_by_key(|(_, run_indication, _)| *run_indication)
                .collect();

            if to_run.len() == 1 {
                let (task_id, _, tick_rate) = to_run[0];
                let time_slice = tick_rate;
                schedule.insert(
                    current_tick..=(current_tick + time_slice - 1),
                    vec![task_id],
                );
                current_tick += time_slice;
                continue;
            }

            // do the different scenarios for how many should run this turn
            match to_run
                .iter()
                .filter(|(_, run_indication, _)| *run_indication == 0)
                .count()
            {
                // Nothing is set to run here
                0 => {
                    current_tick += 1;
                }
                // Full efficient batching
                1 => {
                    let batch_size = to_run[1].2 - to_run[1].1;
                    let (task_id, _, tick_rate) = to_run[0];
                    let normalized_batch_size = batch_size / tick_rate;
                    schedule.insert(
                        current_tick..=(current_tick + normalized_batch_size - 1),
                        vec![task_id],
                    );
                    current_tick += batch_size;
                }
                // Conflicted components
                _ => {
                    schedule.insert(
                        current_tick..=current_tick,
                        to_run
                            .into_iter()
                            .filter_map(|(task_id, run_indication, _)| {
                                if run_indication == 0 {
                                    return Some(task_id);
                                }

                                None
                            })
                            .collect(),
                    );

                    current_tick += 1;
                }
            }
        }

        let tick_real_time = Ratio::new(common_multiple, common_denominator).recip();

        tracing::debug!(
            "Schedule ticks take {:?} and restarts at tick {}, a full cycle takes",
            Duration::from_secs_f64(tick_real_time.to_f64().unwrap()),
            common_denominator,
        );

        Self {
            current_tick: 0,
            rollover_tick: common_denominator,
            tick_real_time,
            schedule,
            allotted_time: Duration::from_secs_f32(Ratio::new(1, 60).to_f32().unwrap()),
            component_store,
            tasks: tasks
                .into_iter()
                .map(|(id, (task, _))| (id, task))
                .collect(),
        }
    }

    pub fn run(&mut self) {
        // TODO: This should actually be calculating how much time is between frames minus draw time
        let mut ticks_this_pass: u64 = 0;
        let timestamp = Instant::now();

        loop {
            let did_overstep_real_time = self.allotted_time < timestamp.elapsed();
            let did_overstep_virtual_time = self.allotted_time.as_secs_f32()
                < (ticks_this_pass as f32 * self.tick_real_time.to_f32().unwrap());

            if did_overstep_virtual_time || did_overstep_real_time {
                break;
            }

            if let Some((time_slice, task_ids)) = self.schedule.get_key_value(&self.current_tick) {
                let ticks = time_slice.clone().count() as u64;

                for task_id in task_ids {
                    if let Some(task_info) = self.tasks.get(task_id) {
                        self.component_store
                            .interact_dyn(task_info.component_id, |component| {
                                let mut task = task_info.task.lock().unwrap();

                                task(component, ticks);
                            });
                    }
                }

                self.current_tick = self.current_tick.wrapping_add(ticks) % self.rollover_tick;
                ticks_this_pass = ticks_this_pass.saturating_add(ticks)
            } else {
                self.current_tick = self.current_tick.wrapping_add(1) % self.rollover_tick;
                ticks_this_pass = ticks_this_pass.saturating_add(1);
            }
        }
    }

    pub fn slow_down(&mut self) {
        // Set our allotted time to lower but not lower than one tick
        self.allotted_time = self
            .allotted_time
            .saturating_sub(Duration::from_nanos(500))
            .max(Duration::from_secs_f32(
                self.tick_real_time.to_f32().unwrap(),
            ));

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
            .min(Duration::from_secs_f32(
                (self.tick_real_time * self.rollover_tick).to_f32().unwrap(),
            ));

        tracing::trace!(
            "Alotted time for scheduler moved up to {:?}",
            self.allotted_time
        );
    }
}
