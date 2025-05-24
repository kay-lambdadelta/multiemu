use crossbeam::channel::{Receiver, Sender};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use super::is_main_thread;

struct QueuedCallback {
    callback: Box<dyn FnOnce() + Send + 'static>,
    is_done: Arc<(Condvar, Mutex<bool>)>,
}

#[derive(Debug)]
pub struct MainThreadQueue {
    sender: Sender<QueuedCallback>,
    receiver: Receiver<QueuedCallback>,
}

impl Default for MainThreadQueue {
    fn default() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();
        Self { sender, receiver }
    }
}

impl MainThreadQueue {
    // Waits for the main thread to execute the callback
    pub fn maybe_wait_on_main<'a>(&'a self, callback: impl FnOnce() + Send + 'a) {
        if is_main_thread() {
            // Just straight up execute it if its the main thread
            callback();
            return;
        }

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
        self.sender
            .send(QueuedCallback {
                callback,
                is_done: is_done.clone(),
            })
            .unwrap();

        let mut is_done_guard = is_done.1.lock().unwrap();
        while !*is_done_guard {
            is_done_guard = is_done.0.wait(is_done_guard).unwrap();
        }
    }

    pub fn main_thread_poll(&self) {
        assert!(is_main_thread());

        loop {
            match self.receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(callback) => {
                    (callback.callback)();
                    let mut is_done_guard = callback.is_done.1.lock().unwrap();
                    *is_done_guard = true;
                    callback.is_done.0.notify_one();
                }
                Err(crossbeam::channel::RecvTimeoutError::Timeout) => break,
                Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                    unreachable!()
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::{MainThreadQueue, set_main_thread};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[test]
    fn test_main_thread_queue() {
        set_main_thread();

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
            queue.main_thread_poll();
            std::hint::spin_loop();
        }

        for worker in workers {
            worker.join().unwrap();
        }

        // Ensure the task executed
        assert_eq!(executed_tasks.load(Ordering::Relaxed), 0);
        assert_eq!(queue.receiver.len(), 0);
    }
}
