use super::Scheduler;
use crate::scheduler::TaskMode;
use std::{num::NonZero, time::Duration};

impl Scheduler {
    /// Runs the scheduler for a single pass
    ///

    pub fn run_for_cycles(&mut self, cycles: u32) {
        let old_current_tick = self.current_tick;

        loop {
            let ticks_passed = self.current_tick.wrapping_sub(old_current_tick);

            if ticks_passed >= cycles {
                break;
            }

            let ticks_left = cycles - ticks_passed;

            // It's OK to use [Option::unwrap_or_default] here; an empty Vec does not allocate in Rust
            let active_events = self
                .active_schedule
                .remove(&self.current_tick)
                .unwrap_or_default();
            let lazy_events = self
                .lazy_schedule
                .remove(&self.current_tick)
                .unwrap_or_default();

            let max_allotted_ticks = self
                .ticks_per_full_cycle
                .min(self.ticks_per_full_cycle - self.current_tick)
                .min(ticks_left)
                .min(
                    self.active_schedule
                        .range(self.current_tick..)
                        .next()
                        .map(|(tick, _)| *tick - self.current_tick)
                        .unwrap_or(self.ticks_per_full_cycle),
                );

            for event in lazy_events {
                let mut task_info = self.storage.tasks[&event].lock().unwrap();

                match &mut task_info.mode {
                    TaskMode::Lazy { debt } => {
                        if let Some(new_debt) = debt.checked_add(1) {
                            *debt = new_debt;
                        } else {
                            *debt = 0;
                            task_info.task.run(NonZero::new(u32::MAX).unwrap());
                        }

                        // Reschedule lazy task
                        self.lazy_schedule
                            .entry(
                                (self.current_tick + task_info.tick_rate)
                                    % self.ticks_per_full_cycle,
                            )
                            .or_default()
                            .push(event);
                    }
                    _ => unreachable!("{:?} {:?}", self, task_info),
                }
            }

            match active_events.len() {
                0 => {}
                1 => {
                    let event = active_events[0];
                    let mut task_info = self.storage.tasks[&event].lock().unwrap();
                    let time_slice =
                        NonZero::new((max_allotted_ticks / task_info.tick_rate).max(1)).unwrap();
                    let representing_time = time_slice.get() * task_info.tick_rate;

                    self.active_schedule
                        .entry((self.current_tick + representing_time) % self.ticks_per_full_cycle)
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(event);

                    task_info.task.run(time_slice);
                    drop(task_info);
                }
                _ => {
                    for event in active_events {
                        let mut task_info = self.storage.tasks[&event].lock().unwrap();
                        let time_slice = NonZero::new(1).unwrap();
                        let representing_time = time_slice.get() * task_info.tick_rate;

                        self.active_schedule
                            .entry(
                                (self.current_tick + representing_time) % self.ticks_per_full_cycle,
                            )
                            .or_insert_with(|| Vec::with_capacity(1))
                            .push(event);

                        task_info.task.run(time_slice);
                        drop(task_info);
                    }
                }
            }

            self.update_current_tick(1);
        }
    }

    pub fn run(&mut self, allotted_duration: Duration) {
        // Do not allow the above runtime to undercut us stupidly
        let allotted_duration = allotted_duration.max(self.tick_real_time);

        let allotted_ticks = (allotted_duration.as_nanos() / self.tick_real_time.as_nanos())
            .try_into()
            .unwrap();

        self.run_for_cycles(allotted_ticks);
    }
}
