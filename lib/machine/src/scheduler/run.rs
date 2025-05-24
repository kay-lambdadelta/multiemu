use super::{Scheduler, TaskInfo};
use std::num::NonZero;

impl Scheduler {
    pub fn run(&mut self) {
        // How many ticks are allowed to be run for this pass,
        let alloted_ticks: u32 = (self.allotted_time.as_nanos() / self.tick_real_time.as_nanos())
            .try_into()
            .unwrap_or(u32::MAX);

        let old_current_tick = self.current_tick;

        // Run until we are out of allotted time
        let mut to_run = Vec::default();

        loop {
            let ticks_passed = self.current_tick.wrapping_sub(old_current_tick);

            if ticks_passed >= alloted_ticks {
                break;
            }

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
                    let next_task_next_execution = next_task_info
                        .next_execution
                        .0
                        .wrapping_sub(self.current_tick);

                    // Don't overstep our alloted time
                    let ticks_to_skip_ahead = next_task_next_execution.min(alloted_ticks);
                    self.current_tick = self.current_tick.wrapping_add(ticks_to_skip_ahead);
                }
                1 => {
                    let task_info = to_run.remove(0);

                    // our heap has two plus tasks
                    if let Some(next_task_info) = self.tasks.peek() {
                        let alloted_ticks = (next_task_info.next_execution.0
                            - task_info.next_execution.0)
                            .min(alloted_ticks);

                        // Fit as many tasks as we can to start
                        if let Some(time_slice) =
                            NonZero::new(alloted_ticks / task_info.tick_rate + 1)
                        {
                            self.run_task([(task_info, time_slice)]);

                            self.current_tick = self.current_tick.wrapping_add(alloted_ticks);
                        } else {
                            // Do not overstep this tasks tickrate
                            let ticks_to_skip_ahead = alloted_ticks.min(task_info.tick_rate);

                            self.run_task([(task_info, NonZero::new(1).unwrap())]);

                            self.current_tick = self.current_tick.wrapping_add(ticks_to_skip_ahead);
                        }
                    // only one task here
                    } else {
                        todo!()
                    }
                }
                _ => {
                    self.run_task(
                        to_run
                            .drain(..)
                            .map(|task_info| (task_info, NonZero::new(1).unwrap())),
                    );

                    self.current_tick = self.current_tick.wrapping_add(1);
                }
            }
        }
    }
}
