use std::{
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crossbeam::atomic::AtomicCell;
pub(crate) use event_queue::{EventQueue, EventType, QueuedEvent};
use fixed::{FixedU128, types::extra::U64};
use rustc_hash::FxBuildHasher;

use crate::component::{ComponentHandle, ComponentPath};

mod event_queue;
#[cfg(test)]
mod tests;

// NOTE: These operations are purely so our minheap schedule timeline works

#[derive(Debug)]
pub struct DrivenComponent {
    component: ComponentHandle,
}

/// The main scheduler that the runtime uses to drive tasks
///
/// It is a frequency based cooperative scheduler with some optional out of
/// order execution stuff
#[derive(Debug)]
pub(crate) struct Scheduler {
    pub event_queue: Arc<EventQueue>,
    driven: HashMap<ComponentPath, DrivenComponent, FxBuildHasher>,
    now: AtomicCell<Period>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            event_queue: Arc::default(),
            driven: HashMap::default(),
            now: AtomicCell::default(),
        }
    }

    pub fn register_driven_component(&mut self, path: ComponentPath, component: ComponentHandle) {
        self.driven.insert(path, DrivenComponent { component });
    }

    pub fn run(&self, allocated_time: Period) {
        let mut now = self.now.load();

        now += allocated_time;
        self.now.store(now);
        self.update_driver_components(now);
    }

    pub fn update_driver_components(&self, now: Period) {
        for driven in self.driven.values() {
            driven.component.interact_mut(now, |_| {})
        }
    }

    pub fn now(&self) -> Period {
        self.now.load()
    }
}

pub type Period = FixedU128<U64>;
pub type Frequency = FixedU128<U64>;

/// Tries to find a reasonable sleep resolution for the dedicated thread based
/// upon several heuristic methods.
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
