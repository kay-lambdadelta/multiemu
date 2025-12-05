use std::{cmp::Reverse, collections::BinaryHeap, fmt::Debug, sync::Mutex};

use crate::{
    component::{Component, ComponentHandle},
    scheduler::{Frequency, Period},
};

#[derive(Debug, Default)]
pub struct EventQueue {
    event_queue: Mutex<BinaryHeap<QueuedEvent>>,
}

impl EventQueue {
    pub fn queue(&self, event: QueuedEvent) {
        self.event_queue.lock().unwrap().push(event);
    }

    pub fn consume_events(&self, upto: Period) {
        let mut queue_guard = self.event_queue.lock().unwrap();

        while let Some(event) = queue_guard.peek() {
            if upto < event.time.0 {
                break;
            }
            let event = queue_guard.pop().unwrap();

            drop(queue_guard);

            match event.ty {
                EventType::Once { callback } => {
                    event.component.interact_mut(event.time.0, |component| {
                        callback(component, event.time.0);
                    });
                    queue_guard = self.event_queue.lock().unwrap();
                }
                EventType::Repeating {
                    frequency,
                    mut callback,
                } => {
                    event.component.interact_mut(event.time.0, |component| {
                        callback(component, event.time.0);
                    });
                    queue_guard = self.event_queue.lock().unwrap();

                    queue_guard.push(QueuedEvent {
                        component: event.component,
                        ty: EventType::Repeating {
                            frequency,
                            callback,
                        },
                        time: Reverse(event.time.0 + frequency.recip()),
                    });
                }
            }
        }
    }

    #[inline]
    pub fn within_deadline(&self, timestamp: Period) -> bool {
        let queue_guard = self.event_queue.lock().unwrap();

        if let Some(next_event) = queue_guard.peek() {
            return next_event.time.0 > timestamp;
        }

        true
    }
}

pub(crate) struct QueuedEvent {
    pub component: ComponentHandle,
    pub ty: EventType,
    pub time: Reverse<Period>,
}

impl PartialEq for QueuedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl Eq for QueuedEvent {}

impl PartialOrd for QueuedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

impl Debug for QueuedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimelineEntry")
            .field("time", &self.time)
            .finish()
    }
}

#[allow(clippy::type_complexity)]
pub(crate) enum EventType {
    Once {
        callback: Box<dyn FnOnce(&mut dyn Component, Period) + Send + Sync>,
    },
    Repeating {
        frequency: Frequency,
        callback: Box<dyn FnMut(&mut dyn Component, Period) + Send + Sync>,
    },
}
