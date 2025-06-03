pub use fragile::Fragile;
pub use main_thread_queue::MainThreadQueue;
use std::{
    cell::OnceCell,
    sync::atomic::{AtomicBool, Ordering},
};

mod fragile;
mod main_thread_queue;

thread_local! {
    static IS_MAIN_THREAD: OnceCell<()> = const { OnceCell::new() };
}
static WAS_MAIN_THREAD_SET: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn is_main_thread() -> bool {
    IS_MAIN_THREAD.with(|is_main_thread| is_main_thread.get().is_some())
}

#[inline]
/// Sets the current thread as the main thread
pub fn set_main_thread() {
    IS_MAIN_THREAD.with(|is_main_thread| {
        let was_set =
            WAS_MAIN_THREAD_SET.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);

        match was_set {
            Ok(_) => {
                let _ = is_main_thread.set(());
            }
            Err(true) => {
                panic!("Another thread was already marked as the main thread");
            }
            Err(false) => unreachable!(),
        }
    });
}

/// Forcefully mark this thread as the main thread
///
/// # Safety
///
/// If this is violated somehow the entire program falls to pieces
///
/// Pleasepleasepleasepleaseplease only use this in tests
#[inline]
pub unsafe fn force_set_main_thread() {
    IS_MAIN_THREAD.with(|is_main_thread| {
        let _ = is_main_thread.set(());
    });
}
