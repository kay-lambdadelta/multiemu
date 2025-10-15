use super::SchedulerState;
use crate::scheduler::ScheduleEntry;
use std::{
    sync::atomic::Ordering,
    thread::sleep,
    time::{Duration, Instant},
};

impl SchedulerState {
    /// Runs the scheduler for X number of passes
    pub fn run_for_cycles(&mut self, cycles: u32) {
        for _ in 0..cycles {
            for ScheduleEntry {
                task_id,
                time_slice,
            } in &self.timeline[self.current_tick as usize]
            {
                let task_info = &mut self.tasks[*task_id as usize];
                (task_info.task)(&self.registry, *time_slice);
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

/// scheduler thread implementation that attempts to run the scheduler actively as much as that would be efficient
pub(crate) fn scheduler_thread(mut state: SchedulerState) {
    let handle = state.handle();

    let reasonable_sleep_resolution = find_reasonable_sleep_resolution().max(state.tick_real_time);
    tracing::info!("Chose sleep resolution: {:?}", reasonable_sleep_resolution);
    let time_block_size = reasonable_sleep_resolution * 2;

    while !handle.exit.load(Ordering::Acquire) {
        if handle.paused.load(Ordering::Acquire) {
            sleep(reasonable_sleep_resolution);
            continue;
        }

        let start = Instant::now();
        state.run(time_block_size);
        let time_taken = start.elapsed();

        let time_to_sleep = time_block_size.saturating_sub(time_taken);

        if time_to_sleep < reasonable_sleep_resolution {
            // Sleeping here is impossible
            continue;
        }

        sleep(time_to_sleep);
    }
}

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
