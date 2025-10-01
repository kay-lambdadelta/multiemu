use super::Scheduler;
use crate::scheduler::ScheduleEntry;
use std::time::Duration;

impl Scheduler {
    /// Runs the scheduler for X number of passes
    pub fn run_for_cycles(&mut self, cycles: u32) {
        for _ in 0..cycles {
            for ScheduleEntry {
                task_id,
                time_slice,
            } in &self.timeline[self.current_tick as usize]
            {
                let task_info = &mut self.tasks[*task_id as usize];
                task_info.task.run(*time_slice);
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
