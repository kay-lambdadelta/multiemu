pub use fragile::Fragile;
pub use main_thread_queue::*;
use std::{sync::OnceLock, thread::ThreadId};

mod fragile;
mod main_thread_queue;

static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();

#[inline]
/// Checks if the current thread is the main thread
pub fn is_main_thread() -> bool {
    if let Some(thread_id) = MAIN_THREAD.get() {
        return *thread_id == std::thread::current().id();
    } else {
        unreachable!("Main thread was not set")
    }
}

#[inline]
/// Sets the current thread as the main thread
///
/// Note that using this incorrectly should not cause unsafety but it will make the program behave badly
///
/// Not using it at all is asking for a crash
pub fn set_main_thread() {
    MAIN_THREAD
        .set(std::thread::current().id())
        .expect("Main thread already set");
}
