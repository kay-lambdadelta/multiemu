use std::{
    cell::OnceCell,
    sync::atomic::{AtomicBool, Ordering},
};

mod fragile;
mod main_thread_queue;

pub use fragile::Fragile;
pub use main_thread_queue::MainThreadQueue;

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
        if is_main_thread.get().is_none() && WAS_MAIN_THREAD_SET.swap(true, Ordering::SeqCst) {
            panic!("Another thread was already marked as the main thread");
        }

        // Ignore multiple attempts to set the main thread from the main thread
        let _ = is_main_thread.set(());
    });
}
