use std::{
    ops::RangeInclusive,
    sync::{Arc, atomic::Ordering},
    thread::sleep,
    time::{Duration, Instant},
};

use multiemu_range::ContiguousRange;

use super::Scheduler;
use crate::scheduler::{SchedulerHandle, TimelineEntry, TimelineTaskEntry};

impl Scheduler {
    /// Runs the scheduler for X number of cycles
    ///
    /// This may not relate to the frequency of any component but represents the number of fine grained steps in a timeline
    pub fn run_for_cycles(&mut self, mut cycles: u32) {
        let timeline = self.timeline.as_mut().unwrap();

        // This particular executor design slices the timeline into efficient runs so we search the timeline as little as possible
        //
        // TODO: If the start and end entry match each other, fuse them

        while cycles != 0 {
            let to_run = (timeline.length - self.current_tick).min(cycles);
            cycles -= to_run;

            let cycle_range = RangeInclusive::from_start_and_length(self.current_tick, to_run);

            for (_, TimelineEntry { tasks, time_slice }) in timeline.entries.range(cycle_range) {
                for TimelineTaskEntry { task_id, component } in tasks {
                    component.interact_mut_with_task(*task_id, |component, task| {
                        (task.callback)(component, *time_slice);
                    });
                }
            }

            self.current_tick = self.current_tick.checked_add(to_run).unwrap() % timeline.length;
        }
    }

    /// Run for a number of cycles closest matching the duration
    ///
    /// Note that the passed in duration is a suggestion and the runtime may run for more or less cycles than requested.
    pub fn run(&mut self, duration: Duration) {
        let timeline = self.timeline.as_mut().unwrap();

        // Do not allow the above runtime to undercut us stupidly
        let allotted_duration = duration.max(timeline.tick_real_time);

        let allotted_ticks = (allotted_duration.as_nanos() / timeline.tick_real_time.as_nanos())
            .try_into()
            .unwrap();

        self.run_for_cycles(allotted_ticks);
    }
}

/// Run the scheduler in a loop, to be called on a dedicated thread
///
/// Currently this is not more efficient than main thread execution
pub(super) fn scheduler_thread(mut state: Scheduler, handle: Arc<SchedulerHandle>) {
    let timeline = state.timeline.as_mut().unwrap();
    let reasonable_sleep_resolution =
        find_reasonable_sleep_resolution().max(timeline.tick_real_time);

    tracing::info!(
        "Chose sleep resolution for the dedicated thread: {:?}",
        reasonable_sleep_resolution
    );
    // NOTE: This is rather arbitrary
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
