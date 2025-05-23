use crate::component::{Component, ComponentId, store::ComponentStore};
use num::{
    ToPrimitive,
    integer::{gcd, lcm},
    rational::Ratio,
};
use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    fmt::Debug,
    num::NonZero,
    sync::{Arc, Mutex},
    time::Duration,
};

struct TaskInfo {
    pub component_id: ComponentId,
    #[allow(clippy::type_complexity)]
    pub task: Mutex<Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>>,
    pub tick_rate: u32,
    pub next_execution: Reverse<u32>,
}

// Used ONLY for the binary heap
impl PartialEq for TaskInfo {
    fn eq(&self, other: &Self) -> bool {
        self.next_execution == other.next_execution
    }
}

impl Eq for TaskInfo {}

impl PartialOrd for TaskInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.next_execution.cmp(&other.next_execution))
    }
}

impl Ord for TaskInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.next_execution.cmp(&other.next_execution)
    }
}

impl Debug for TaskInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredTask")
            .field("component_id", &self.component_id)
            .field("tick_rate", &self.tick_rate)
            .field("next_execution", &self.next_execution)
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
    tasks: BinaryHeap<TaskInfo>,
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
        pub struct PrecalculationTask {
            pub component_id: ComponentId,
            #[allow(clippy::type_complexity)]
            pub task: Mutex<Box<dyn FnMut(&dyn Component, NonZero<u32>) + Send + 'static>>,
            pub frequency: Ratio<u32>,
        }

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

                PrecalculationTask {
                    task: Mutex::new(task),
                    component_id,
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

        tracing::info!("System frequency is {}", common);

        let tick_real_time = Duration::from_secs_f64(common.to_f64().unwrap());
        let rollover_tick = common.recip().to_integer();

        tracing::debug!(
            "Schedule ticks take {:?} and rolls over at tick {}, a full cycle takes {:?}",
            tick_real_time,
            rollover_tick,
            tick_real_time * rollover_tick as u32
        );

        let tasks = tasks
            .into_iter()
            .map(|precalcuation_task| {
                let factor = rollover_tick / precalcuation_task.frequency.denom();
                let tick_rate = precalcuation_task.frequency.numer() * factor;

                TaskInfo {
                    component_id: precalcuation_task.component_id,
                    task: precalcuation_task.task,
                    tick_rate,
                    next_execution: Reverse(0),
                }
            })
            .collect();

        tracing::debug!("Tasks {:#?}", tasks);

        Self {
            current_tick: 0,
            tick_real_time,
            rollover_tick,
            allotted_time: Duration::from_secs(1) / 60,
            component_store,
            tasks,
        }
    }

    pub fn run(&mut self) {
        let alloted_ticks_this_pass: u32 = (self.allotted_time.as_nanos()
            / self.tick_real_time.as_nanos())
        .try_into()
        .unwrap();
        let old_current_tick = self.current_tick;

        // Run until we are out of allotted time
        let mut to_run = Vec::default();

        while self.current_tick.wrapping_sub(old_current_tick) < alloted_ticks_this_pass {
            // Peek to see if the min task is ready
            while let Some(TaskInfo { next_execution, .. }) = self.tasks.peek() {
                assert!(
                    next_execution.0 >= self.current_tick,
                    "Task counter and global counter desync: {} vs {}",
                    next_execution.0,
                    self.current_tick
                );

                if next_execution.0 != self.current_tick {
                    break;
                }

                to_run.push(self.tasks.pop().unwrap());
            }

            match to_run.len() {
                0 => {
                    // Timeskip to the nearest task
                    let next_task_info = self.tasks.peek().unwrap();
                    let ticks_to_skip_ahead = next_task_info
                        .next_execution
                        .0
                        .wrapping_sub(self.current_tick);

                    self.current_tick = self.current_tick.wrapping_add(ticks_to_skip_ahead);
                }
                1 => {
                    let task_info = to_run.remove(0);
                    let next_task_info = self.tasks.peek().unwrap();

                    let mut ticks_until_next_task =
                        next_task_info.next_execution.0 - task_info.next_execution.0;

                    if let Some(time_slice) =
                        NonZero::new(ticks_until_next_task / task_info.tick_rate)
                    {
                        self.run_task([(task_info, time_slice)]);
                    } else {
                        // Reschedule next task up to give this task some wiggle room
                        let adjustment = task_info.tick_rate - ticks_until_next_task;
                        let mut next_task_info = self.tasks.peek_mut().unwrap();

                        next_task_info.next_execution.0 =
                            next_task_info.next_execution.0.wrapping_add(adjustment);
                        ticks_until_next_task += adjustment;

                        // Allow the binary heap to recompute itself
                        drop(next_task_info);

                        self.run_task([(task_info, NonZero::new(1).unwrap())]);
                    }

                    self.current_tick = self.current_tick.wrapping_add(ticks_until_next_task);
                }
                _ => {
                    let lcm_tick_rate = to_run
                        .iter()
                        .map(|task_info| task_info.tick_rate)
                        .fold(1, lcm);

                    if let Some(next_task_info) = self.tasks.peek() {
                        let mut ticks_until_next_task =
                            next_task_info.next_execution.0 - (self.current_tick + lcm_tick_rate);

                        // Make sure the slowest task fits
                        if (ticks_until_next_task / lcm_tick_rate) != 0 {
                            // Its ensured everything fits here since the max fits
                            self.run_task(to_run.drain(..).map(|task_info| {
                                let time_slice =
                                    NonZero::new(ticks_until_next_task / task_info.tick_rate)
                                        .unwrap();

                                (task_info, time_slice)
                            }));
                        } else {
                            // Reschedule next task up to give this task some wiggle room
                            let adjustment = lcm_tick_rate - ticks_until_next_task;
                            let mut next_task_info = self.tasks.peek_mut().unwrap();

                            next_task_info.next_execution.0 =
                                next_task_info.next_execution.0.wrapping_add(adjustment);
                            ticks_until_next_task += adjustment;

                            // Allow the binary heap to recompute itself
                            drop(next_task_info);

                            self.run_task(to_run.drain(..).map(|task_info| {
                                let time_slice =
                                    NonZero::new(ticks_until_next_task / task_info.tick_rate)
                                        .unwrap();

                                (task_info, time_slice)
                            }));
                        }

                        // Update by the advancement to the next task
                        self.current_tick = self.current_tick.wrapping_add(ticks_until_next_task);
                    } else {
                        // Just run for the max tick rates period since apparently the whole set is here
                        self.run_task(to_run.drain(..).map(|task_info| {
                            let time_slice =
                                NonZero::new(lcm_tick_rate / task_info.tick_rate).unwrap();

                            (task_info, time_slice)
                        }));

                        // Update by max tick rate
                        self.current_tick = self.current_tick.wrapping_add(lcm_tick_rate);
                    };
                }
            }
        }
    }

    fn run_task(&mut self, to_run: impl IntoIterator<Item = (TaskInfo, NonZero<u32>)>) {
        for (mut task_info, time_slice) in to_run {
            self.component_store
                .interact_dyn_local(task_info.component_id, |component| {
                    let mut task = task_info.task.lock().unwrap();

                    task(component, time_slice);
                })
                .unwrap();

            // Update the next execution
            let ticks_taken = time_slice.get() * task_info.tick_rate;
            task_info.next_execution.0 = task_info.next_execution.0.wrapping_add(ticks_taken);

            // Put it back on our heap and let it rearrange itself
            self.tasks.push(task_info);
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
