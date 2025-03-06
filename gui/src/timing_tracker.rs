use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::time::{Duration, Instant};

#[derive(Clone)]
/// Keeps track of the timing of the last few frames
pub struct TimingTracker {
    last_starting_frame: Option<Instant>,
    recent_frame_timings: AllocRingBuffer<Duration>,
}

impl Default for TimingTracker {
    fn default() -> Self {
        Self {
            last_starting_frame: None,
            recent_frame_timings: AllocRingBuffer::new(32),
        }
    }
}

impl TimingTracker {
    pub fn reset_frame_timings(&mut self) {
        self.recent_frame_timings.clear();
    }

    pub fn machine_main_cycle_starting(&mut self) {
        self.last_starting_frame = Some(Instant::now());
    }

    pub fn machine_main_cycle_ending(&mut self) -> Duration {
        let now = Instant::now();

        let time_taken = now.saturating_duration_since(
            self.last_starting_frame
                .take()
                .expect("Frame ending called before the frame started"),
        );
        self.recent_frame_timings.push(time_taken);

        time_taken
    }

    pub fn average_timings(&self) -> Duration {
        self.recent_frame_timings
            .iter()
            .sum::<Duration>()
            .checked_div(self.recent_frame_timings.len() as u32)
            .unwrap_or_default()
    }
}
