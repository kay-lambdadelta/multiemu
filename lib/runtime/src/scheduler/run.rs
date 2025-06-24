use super::Scheduler;
use crate::scheduler::TaskToExecute;
use std::{num::NonZero, time::Duration};

impl Scheduler {
    /// Runs the scheduler for a single pass
    ///
    pub fn run(&mut self, allotted_duration: Duration) {
        // Do not allow the above runtime to undercut us stupidly
        let allotted_duration = allotted_duration.max(self.tick_real_time);
        let old_current_tick = self.current_tick;

        let allotted_ticks = (allotted_duration.as_nanos() / self.tick_real_time.as_nanos())
            .try_into()
            .unwrap();

        loop {
            let ticks_passed = self.current_tick.wrapping_sub(old_current_tick);

            if ticks_passed >= allotted_ticks {
                break;
            }

            let ticks_left = allotted_ticks - ticks_passed;

            // It's OK to use [Option::unwrap_or_default] here; an empty Vec does not allocate in Rust
            let events = self.schedule.remove(&self.current_tick).unwrap_or_default();
            // Try to form a execution boundary
            let max_allotted_ticks = self
                .schedule
                .range(self.current_tick..)
                .next()
                // Until the next event
                .map(|(event_location, _)| event_location - self.current_tick)
                // Use the boundary from a full cycle as a placeholder if there is no next event
                .unwrap_or(self.ticks_per_full_cycle)
                // Do not overstep the cycle boundary
                .min(self.ticks_per_full_cycle - self.current_tick)
                // Do not overstep our allotted ticks
                .min(ticks_left);

            match events.len() {
                0 => {
                    self.update_current_tick(max_allotted_ticks);
                }
                1 => {
                    let event = &events[0];
                    let event_info = self.tasks.get(&event).unwrap();

                    self.run_tasks([TaskToExecute {
                        id: *event,
                        time_slice: NonZero::new(
                            (max_allotted_ticks / event_info.tick_rate).max(1),
                        )
                        .unwrap(),
                    }]);
                }
                _ => {
                    self.run_tasks(events.into_iter().map(|event| TaskToExecute {
                        id: event,
                        time_slice: NonZero::new(1).unwrap(),
                    }));
                }
            }
        }
    }
}
