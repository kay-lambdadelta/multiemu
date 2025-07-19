use super::Scheduler;
use crate::scheduler::{ScheduleEntry, TaskMode};
use std::{num::NonZero, time::Duration};

impl Scheduler {
    /// Runs the scheduler for X number of passes
    pub fn run_for_cycles(&mut self, cycles: u32) {
        for _ in 0..cycles {
            for ScheduleEntry {
                task_id,
                time_slice,
            } in &self.timeline[self.current_tick as usize]
            {
                let mut task_info = self.storage.tasks.get(task_id).unwrap().lock().unwrap();
                let task_info = &mut *task_info;

                match &mut task_info.mode {
                    TaskMode::Active => {
                        task_info.task.run(*time_slice);
                    }
                    TaskMode::Lazy { debt } => {
                        if let Some(new_debt) = debt.checked_add(time_slice.get()) {
                            *debt = new_debt;
                        } else {
                            task_info.task.run(NonZero::new(u32::MAX).unwrap());
                            *debt %= time_slice.get();
                        }
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
