use std::cell::OnceCell;

mod fragile;
mod main_thread_queue;

pub use fragile::Fragile;
pub use main_thread_queue::MainThreadQueue;

thread_local! {
    static IS_MAIN_THREAD: OnceCell<()> = const { OnceCell::new() };
}

#[inline]
pub fn is_main_thread() -> bool {
    IS_MAIN_THREAD.with(|is_main_thread| is_main_thread.get().is_some())
}

#[inline]
/// Sets the current thread as the main thread
/// 
/// # Safety
/// 
/// If this gets called from more than one thread several pieces of this framework will become unsound
pub unsafe fn set_main_thread() {
    IS_MAIN_THREAD.with(|is_main_thread| is_main_thread.set(()).unwrap());
}
