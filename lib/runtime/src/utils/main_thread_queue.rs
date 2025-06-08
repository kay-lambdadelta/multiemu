use super::is_main_thread;
use std::{
    any::Any,
    collections::VecDeque,
    fmt::Debug,
    ops::Deref,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

#[allow(clippy::type_complexity)]
struct QueuedCallback {
    callback: Box<dyn FnOnce() -> Box<dyn Any + Send> + Send + 'static>,
    is_done: Arc<(Condvar, Mutex<Option<Box<dyn Any + Send>>>)>,
}

#[derive(Default)]
pub struct MainThreadQueue {
    queue: Mutex<VecDeque<QueuedCallback>>,
    global_condvar: Condvar,
}

impl Debug for MainThreadQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MainThreadQueue").finish()
    }
}

impl MainThreadQueue {
    // Waits for the main thread to execute the callback
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
            let callback_return = callback();

            Box::new(callback_return) as Box<dyn Any + Send>
        });

        // lifetime extend the callback
        let callback = unsafe {
            std::mem::transmute::<
                Box<dyn FnOnce() -> Box<dyn Any + Send> + Send + 'a>,
                Box<dyn FnOnce() -> Box<dyn Any + Send> + Send + 'static>,
            >(callback)
        };

        let is_done = Arc::new((Condvar::default(), Mutex::default()));

        let mut callback_queue_guard = self.queue.lock().unwrap();
        callback_queue_guard.push_back(QueuedCallback {
            callback,
            is_done: is_done.clone(),
        });
        drop(callback_queue_guard);
        self.global_condvar.notify_one();

        let mut is_done_guard = is_done.1.lock().unwrap();
        loop {
            match is_done_guard.deref() {
                Some(_) => {
                    let value = is_done_guard.take().unwrap();

                    return *value.downcast().unwrap();
                }
                None => is_done_guard = is_done.0.wait(is_done_guard).unwrap(),
            }
        }
    }

    pub fn main_thread_poll(&self, timeout: Duration) {
        assert!(is_main_thread());

        let callback_queue_guard = self.queue.lock().unwrap();

        let (mut callback_queue_guard, _) = self
            .global_condvar
            .wait_timeout_while(callback_queue_guard, timeout, |callback_queue_guard| {
                callback_queue_guard.is_empty()
            })
            .unwrap();

        if let Some(callback) = callback_queue_guard.pop_front() {
            let value = (callback.callback)();
            let mut is_done_guard = callback.is_done.1.lock().unwrap();
            *is_done_guard = Some(value);
            callback.is_done.0.notify_one();
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::MainThreadQueue;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[test]
    fn test_main_thread_queue() {
        unsafe { crate::utils::force_set_main_thread() };

        let queue = Arc::new(MainThreadQueue::default());
        let tasks_to_run = 10;
        let executed_tasks = Arc::new(AtomicUsize::new(tasks_to_run));
        let mut workers = Vec::new();

        for _ in 0..tasks_to_run {
            workers.push(std::thread::spawn({
                let queue = queue.clone();
                let executed_tasks = executed_tasks.clone();

                move || {
                    queue.maybe_wait_on_main(|| {
                        std::thread::sleep(Duration::from_secs(1));
                        executed_tasks.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }));
        }

        // For our purposes here we want to check the receiver but otherwise it would just be polled forever
        while executed_tasks.load(Ordering::Relaxed) != 0 {
            queue.main_thread_poll(Duration::from_millis(1));
        }

        for worker in workers {
            worker.join().unwrap();
        }

        // Ensure the task executed
        assert_eq!(executed_tasks.load(Ordering::Relaxed), 0);
    }
}
