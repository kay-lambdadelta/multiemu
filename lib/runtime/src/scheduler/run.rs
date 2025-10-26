use super::Scheduler;
use crate::scheduler::{ScheduleEntry, TaskType};
use std::{
    num::NonZero,
    sync::atomic::Ordering,
    thread::sleep,
    time::{Duration, Instant},
};

impl Scheduler {
    /// Runs the scheduler for X number of passes
    pub fn run_for_cycles(&mut self, cycles: u32) {
        let timeline = self.timeline.as_mut().unwrap();

        for _ in 0..cycles {
            for ScheduleEntry {
                task_id,
                time_slice,
                component,
            } in &timeline.entries[self.current_tick as usize]
            {
                component.interact_mut_with_task(*task_id, |component, task| {
                    // FIXME: Verify logic edge cases

                    match task.ty {
                        // Direct tasks never have debt
                        TaskType::Direct => {
                            (task.callback)(component, *time_slice);
                        }
                        TaskType::Lazy => {
                            let (new_debt, overflowed) =
                                task.debt.overflowing_add(time_slice.get());
                            task.debt = new_debt;

                            if overflowed {
                                (task.callback)(component, NonZero::new(u32::MAX).unwrap());
                            }
                        }
                    }
                });
            }

            self.current_tick =
                self.current_tick.checked_add(1).unwrap() % timeline.entries.len() as u32;
        }
    }

    pub fn run(&mut self, allotted_duration: Duration) {
        let timeline = self.timeline.as_mut().unwrap();

        // Do not allow the above runtime to undercut us stupidly
        let allotted_duration = allotted_duration.max(timeline.tick_real_time);

        let allotted_ticks = (allotted_duration.as_nanos() / timeline.tick_real_time.as_nanos())
            .try_into()
            .unwrap();

        self.run_for_cycles(allotted_ticks);
    }
}

/// scheduler thread implementation that attempts to run the scheduler actively as much as that would be efficient
pub(super) fn scheduler_thread(mut state: Scheduler) {
    let timeline = state.timeline.as_mut().unwrap();
    let reasonable_sleep_resolution =
        find_reasonable_sleep_resolution().max(timeline.tick_real_time);

    let handle = state.handle();

    tracing::info!(
        "Chose sleep resolution for the dedicated thread: {:?}",
        reasonable_sleep_resolution
    );
    let time_block_size = reasonable_sleep_resolution * 2;
    let mut sleep_debt = Duration::ZERO;

    while !handle.exit.load(Ordering::Acquire) {
        if handle.paused.load(Ordering::Acquire) {
            sleep(reasonable_sleep_resolution);
            continue;
        }

        let start = Instant::now();
        state.run(time_block_size);
        let time_taken = start.elapsed();

        let time_to_sleep = time_block_size.saturating_sub(time_taken);
        sleep_debt += time_to_sleep;

        if sleep_debt > reasonable_sleep_resolution {
            sleep(sleep_debt);
            sleep_debt = Duration::ZERO;
        }
    }
}

/// Tries to find a reasonable sleep resolution for the dedicated thread based upon several heuristic methods.
fn find_reasonable_sleep_resolution() -> Duration {
    let min_exponent = 10;
    // ~32 milliseconds, if the system can't keep up with this, oh well
    let max_exponent = 25;
    let trials = 5;

    for exponent in min_exponent..=max_exponent {
        let nanos = 2u64.pow(exponent);
        let duration = Duration::from_nanos(nanos);
        let mut total_error = Duration::ZERO;

        for _ in 0..trials {
            let start = Instant::now();

            sleep(duration);
            let time_taken = start.elapsed();

            let error = time_taken.abs_diff(duration);
            total_error += error;
        }

        let avg_error = total_error / trials;
        if avg_error <= duration / 10 {
            return duration;
        }
    }

    Duration::from_millis(16)
}
