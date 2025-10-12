pub use fragile::Fragile;
pub use main_thread_queue::*;
use std::{
    cell::LazyCell,
    num::NonZero,
    ops::Deref,
    sync::{
        OnceLock,
        atomic::{AtomicU32, Ordering},
    },
};

mod fragile;
mod main_thread_queue;

type ThreadId = NonZero<u32>;

static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();
static MAIN_THREAD_COUNTER: AtomicU32 = AtomicU32::new(1);

thread_local! {
    static THREAD_ID: LazyCell<ThreadId> = LazyCell::new(||  {
        let id = MAIN_THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
        NonZero::new(id).expect("Main thread counter overflowed")
    });
}

#[inline]
/// Checks if the current thread is the main thread
pub fn is_main_thread() -> bool {
    THREAD_ID.with(|id| MAIN_THREAD.get() == Some(id.deref()))
}

#[inline]
/// Sets the current thread as the main thread
///
/// Note that using this incorrectly should not cause unsafety but it will make the program behave badly
///
/// Not using it at all is asking for a crash
pub fn set_main_thread() {
    MAIN_THREAD
        .set(THREAD_ID.with(|id| *id.deref()))
        .expect("Main thread already set");
}
