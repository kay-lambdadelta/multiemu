use super::store::IS_MAIN_THREAD;
use crossbeam::queue::SegQueue;
use std::sync::{Arc, Condvar, Mutex};

struct QueuedCallback {
    callback: Box<dyn FnOnce() + Send + 'static>,
    is_done: Arc<(Condvar, Mutex<bool>)>,
}

#[derive(Default, Debug)]
pub struct MainThreadExecutor(SegQueue<QueuedCallback>);

impl MainThreadExecutor {
    pub fn wait_on_main<'a>(&'a self, callback: impl FnOnce() + Send + 'a) {
        IS_MAIN_THREAD.with(|is_main_thread| {
            assert!(!*is_main_thread.borrow(), "This is for workers only");
        });

        // Box the callback
        let callback = Box::new(callback) as Box<dyn FnOnce() + Send>;

        // lifetime extend the callback
        let callback = unsafe {
            std::mem::transmute::<Box<dyn FnOnce() + Send + 'a>, Box<dyn FnOnce() + Send + 'static>>(
                callback,
            )
        };

        let is_done = Arc::new((Condvar::default(), Mutex::new(false)));

        // Put it on the queue
        self.0.push(QueuedCallback {
            callback,
            is_done: is_done.clone(),
        });

        let mut is_done_guard = is_done.1.lock().unwrap();

        while !*is_done_guard {
            is_done_guard = is_done.0.wait(is_done_guard).unwrap();
        }
    }

    pub fn main_thread_poll(&self) {
        IS_MAIN_THREAD.with(|is_main_thread| {
            assert!(
                *is_main_thread.borrow(),
                "main_thread_poll must be called on the main thread"
            );
        });

        while let Some(callback) = self.0.pop() {
            (callback.callback)();
            let mut is_done_guard = callback.is_done.1.lock().unwrap();
            *is_done_guard = true;
            callback.is_done.0.notify_one();
        }
    }
}

#[cfg(test)]
mod test {
    use crate::component::{main_thread_queue::MainThreadExecutor, store::IS_MAIN_THREAD};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[test]
    fn test_main_thread_queue() {
        IS_MAIN_THREAD.with(|is_main_thread| *is_main_thread.borrow_mut() = true);

        let queue = Arc::new(MainThreadExecutor::default());
        let executed_tasks = Arc::new(AtomicUsize::new(0));

        let left_to_execute = executed_tasks.clone();

        let worker = std::thread::spawn({
            let queue = queue.clone();
            let left_to_execute = left_to_execute.clone();
            left_to_execute.fetch_add(1, Ordering::SeqCst);

            move || {
                queue.wait_on_main(|| {
                    std::thread::sleep(Duration::from_millis(100));
                });

                left_to_execute.fetch_sub(1, Ordering::SeqCst);
            }
        });

        while left_to_execute.load(Ordering::SeqCst) != 0 {
            queue.main_thread_poll();
            std::thread::yield_now();
        }

        worker.join().unwrap();

        // Ensure the task executed
        assert_eq!(executed_tasks.load(Ordering::SeqCst), 0);
        assert_eq!(queue.0.len(), 0);
    }
}
