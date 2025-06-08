use crate::scheduler::wait_on_inflight;

use super::{Scheduler, TaskInfo, run_task};
use std::num::NonZero;

impl Scheduler {
    pub fn run(&mut self) {
        if let Some(barrier) = self.barrier.take() {
            // Make everyone run
            barrier.wait();
        }

        // How many ticks are allowed to be run for this pass,
        let alloted_ticks: u32 = (self.allotted_time.as_nanos() / self.tick_real_time.as_nanos())
            .try_into()
            .unwrap_or(u32::MAX);

        let old_current_tick = self.current_tick;

        loop {
            let ticks_passed = self.current_tick.wrapping_sub(old_current_tick);

            if ticks_passed >= alloted_ticks {
                break;
            }

            // Peek to see if the min task is ready
            while let Some(TaskInfo { next_execution, .. }) = self.tasks.peek() {
                if next_execution.0 != self.current_tick {
                    break;
                }

                self.to_run.push(self.tasks.pop().unwrap());
            }

            match self.to_run.len() {
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
                    let task_info = self.to_run.remove(0);

                    // our heap has two plus tasks
                    if let Some(next_task_info) = self.tasks.peek() {
                        let alloted_ticks = (next_task_info.next_execution.0
                            - task_info.next_execution.0)
                            .min(alloted_ticks);

                        // Fit as many tasks as we can to start
                        if let Some(time_slice) =
                            NonZero::new(alloted_ticks / task_info.tick_rate + 1)
                        {
                            run_task(
                                [(task_info, time_slice)],
                                &mut self.tasks,
                                &mut self.inflight,
                                &self.main_thread_queue,
                                self.tick_real_time,
                            );

                            self.current_tick = self.current_tick.wrapping_add(alloted_ticks);
                        } else {
                            // Do not overstep this tasks tickrate
                            let ticks_to_skip_ahead = alloted_ticks.min(task_info.tick_rate);

                            run_task(
                                [(task_info, NonZero::new(1).unwrap())],
                                &mut self.tasks,
                                &mut self.inflight,
                                &self.main_thread_queue,
                                self.tick_real_time,
                            );

                            self.current_tick = self.current_tick.wrapping_add(ticks_to_skip_ahead);
                        }
                    // only one task here
                    } else {
                        todo!()
                    }
                }
                _ => {
                    run_task(
                        self.to_run
                            .drain(..)
                            .map(|task_info| (task_info, NonZero::new(1).unwrap())),
                        &mut self.tasks,
                        &mut self.inflight,
                        &self.main_thread_queue,
                        self.tick_real_time,
                    );

                    self.current_tick = self.current_tick.wrapping_add(1);
                }
            }
        }

        wait_on_inflight(
            &mut self.inflight,
            &self.main_thread_queue,
            self.tick_real_time,
        )
    }
}
