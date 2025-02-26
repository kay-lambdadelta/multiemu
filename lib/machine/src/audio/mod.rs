use crossbeam::queue::ArrayQueue;
use std::sync::{Arc, atomic::AtomicI16};

pub struct AudioQueue {
    pub queue: ArrayQueue<i16>,
    pub last_seen_sample: AtomicI16,
}

