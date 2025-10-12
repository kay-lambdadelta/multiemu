use super::is_main_thread;
use std::{any::Any, fmt::Debug, sync::Arc};

/// Main thread callback
pub type MainThreadCallback = Box<dyn FnOnce() -> Box<dyn Any + Send> + Send + 'static>;

/// Executor for the main thread
pub trait MainThreadExecutor: Send + Sync + 'static {
    /// Blocks until callback is executed and returns the result
    fn execute(&self, callback: MainThreadCallback) -> Box<dyn Any + Send>;
}

/// Tool that allows blocking on the main thread for the completion of a callback
pub struct MainThreadQueue {
    // TODO: Make this generic
    executor: Arc<dyn MainThreadExecutor>,
}

impl Debug for MainThreadQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainThreadQueue").finish()
    }
}

impl MainThreadQueue {
    /// Creates a new main thread queue
    pub fn new(executor: Arc<impl MainThreadExecutor>) -> Arc<Self> {
        Self { executor }.into()
    }

    /// Waits for the main thread to execute the callback
    ///
    /// This will block if we are not on the main thread, possibly causing a performance loss
    ///
    /// Keep this in mind
    #[inline]
    pub fn maybe_wait_on_main<'a, T: Send + 'static>(
        &'a self,
        callback: impl FnOnce() -> T + Send + 'a,
    ) -> T {
        if is_main_thread() {
            // Just straight up execute it if its the main thread
            return callback();
        }

        // Box the callback and erase the type
        let callback = Box::new(move || {
            assert!(is_main_thread());

            let callback_return = callback();

            Box::new(callback_return) as Box<dyn Any + Send>
        });

        // lifetime extend the callback
        let callback = unsafe {
            std::mem::transmute::<
                Box<dyn FnOnce() -> Box<dyn Any + Send> + Send + 'a>,
                MainThreadCallback,
            >(callback)
        };

        let value = self.executor.execute(callback);

        *value.downcast().unwrap()
    }
}

// FIXME: This effectively breaks tests if they arent singlethreaded

/// Just executes the callback here
///
/// This is useful for testing
pub struct DirectMainThreadExecutor;

impl MainThreadExecutor for DirectMainThreadExecutor {
    fn execute(&self, callback: MainThreadCallback) -> Box<dyn Any + Send> {
        callback()
    }
}
