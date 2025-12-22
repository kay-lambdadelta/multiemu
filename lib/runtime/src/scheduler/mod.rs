use std::{
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crossbeam::atomic::AtomicCell;
pub(crate) use event::{EventManager, EventType, PreemptionSignal, QueuedEvent};
use fixed::{FixedU128, types::extra::U64};
use rustc_hash::FxBuildHasher;

use crate::{component::ComponentHandle, path::FluxEmuPath};

mod event;
#[cfg(test)]
mod tests;

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
    pub event_queue: Arc<EventManager>,
    driven: HashMap<FluxEmuPath, DrivenComponent, FxBuildHasher>,
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

    pub fn register_driven_component(&mut self, path: FluxEmuPath, component: ComponentHandle) {
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

#[derive(Debug)]
pub struct SynchronizationContext<'a> {
    pub(crate) event_manager: &'a EventManager,
    pub(crate) updated_timestamp: &'a mut Period,
    pub(crate) target_timestamp: Period,
    pub(crate) last_attempted_allocation: &'a mut Option<Period>,
    pub(crate) interrupt: &'a PreemptionSignal,
}

impl<'a> SynchronizationContext<'a> {
    #[inline]
    pub fn allocate<'b>(
        &'b mut self,
        period: Period,
        execution_limit: Option<u64>,
    ) -> QuantaIterator<'b, 'a> {
        *self.last_attempted_allocation = Some(period);

        let mut stop_time = self.target_timestamp;

        if let Some(next_event) = self.event_manager.next_event() {
            stop_time = stop_time.min(next_event)
        }

        let mut budget = (stop_time.saturating_sub(*self.updated_timestamp) / period)
            .floor()
            .to_num::<u64>();

        if let Some(execution_limit) = execution_limit {
            budget = budget.min(execution_limit);
        }

        QuantaIterator {
            period,
            budget,
            context: self,
        }
    }
}

pub struct QuantaIterator<'b, 'a> {
    period: Period,
    budget: u64,
    context: &'b mut SynchronizationContext<'a>,
}

impl Iterator for QuantaIterator<'_, '_> {
    type Item = Period;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // New event(s) spotted we have not evaluated
        while self.context.interrupt.needs_preemption() {
            let mut stop_time = self.context.target_timestamp;

            if let Some(next_event) = self.context.event_manager.next_event() {
                stop_time = stop_time.min(next_event)
            }

            let new_budget = (stop_time.saturating_sub(*self.context.updated_timestamp)
                / self.period)
                .floor()
                .to_num::<u64>();

            self.budget = self.budget.min(new_budget);
        }

        if self.budget == 0 {
            return None;
        } else {
            self.budget -= 1;
        }

        let next_timestamp = *self.context.updated_timestamp + self.period;
        *self.context.updated_timestamp = next_timestamp;
        Some(next_timestamp)
    }
}
