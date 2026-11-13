//! Running analyses without freezing the interface.
//!
//! On the desktop a job runs on its own thread. A web page has a single
//! thread, so there a job runs when it is first polled after having been
//! announced, which lets the interface show that work has started.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

/// Shared progress of a running job.
#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicUsize,
    total: AtomicUsize,
    cancelled: AtomicBool,
}

impl Progress {
    /// Report progress. Returns `false` once the job has been cancelled, at
    /// which point the work should stop.
    pub fn report(&self, done: usize, total: usize) -> bool {
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        !self.cancelled.load(Ordering::Relaxed)
    }

    /// Fraction complete, from 0 to 1.
    pub fn fraction(&self) -> f32 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            (self.done.load(Ordering::Relaxed) as f32 / total as f32).clamp(0.0, 1.0)
        }
    }
}

/// A computation whose result arrives later.
pub struct Job<T> {
    inner: imp::Inner<T>,
    progress: Arc<Progress>,
}

impl<T> Job<T> {
    pub fn fraction(&self) -> f32 {
        self.progress.fraction()
    }

    /// Ask the job to stop. Its result, if any, is discarded when the job is
    /// dropped.
    pub fn cancel(&self) {
        self.progress.cancelled.store(true, Ordering::Relaxed);
    }
}

impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        self.progress.cancelled.store(true, Ordering::Relaxed);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use std::sync::mpsc::{channel, Receiver, TryRecvError};

    pub struct Inner<T>(Receiver<T>);

    impl<T: Send + 'static> Job<T> {
        /// Start `work` on another thread. It is given the job's progress
        /// handle to report to and to learn of cancellation from.
        pub fn spawn(work: impl FnOnce(&Progress) -> T + Send + 'static) -> Job<T> {
            let (sender, receiver) = channel();
            let progress = Arc::new(Progress::default());
            let shared = Arc::clone(&progress);
            std::thread::spawn(move || {
                // The receiver is gone if the job was dropped; that is fine.
                let _ = sender.send(work(&shared));
            });
            Job {
                inner: Inner(receiver),
                progress,
            }
        }

        /// The result, once the job has finished. `Err(true)` while it is
        /// still running; `Err(false)` if it ended without a result.
        pub fn poll(&self) -> Result<T, bool> {
            match self.inner.0.try_recv() {
                Ok(value) => Ok(value),
                Err(TryRecvError::Empty) => Err(true),
                Err(TryRecvError::Disconnected) => Err(false),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use std::cell::{Cell, RefCell};

    type Work<T> = Box<dyn FnOnce(&Progress) -> T>;

    pub struct Inner<T> {
        work: RefCell<Option<Work<T>>>,
        announced: Cell<bool>,
    }

    impl<T: 'static> Job<T> {
        /// Queue `work`. It runs on the page's only thread, the second time
        /// the job is polled.
        pub fn spawn(work: impl FnOnce(&Progress) -> T + 'static) -> Job<T> {
            Job {
                inner: Inner {
                    work: RefCell::new(Some(Box::new(work))),
                    announced: Cell::new(false),
                },
                progress: Arc::new(Progress::default()),
            }
        }

        /// The result, once the job has finished. `Err(true)` while it is
        /// still pending; `Err(false)` if it has already been collected.
        pub fn poll(&self) -> Result<T, bool> {
            // Report "running" once first, so a frame is drawn saying so.
            if !self.inner.announced.replace(true) {
                return Err(true);
            }
            match self.inner.work.borrow_mut().take() {
                Some(work) => Ok(work(&self.progress)),
                None => Err(false),
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait<T: Send + 'static>(job: &Job<T>) -> Option<T> {
        let start = Instant::now();
        loop {
            match job.poll() {
                Ok(value) => return Some(value),
                Err(false) => return None,
                Err(true) if start.elapsed() > Duration::from_secs(10) => {
                    panic!("job did not finish")
                }
                Err(true) => std::thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    #[test]
    fn returns_its_result_and_reports_progress() {
        let job = Job::spawn(|progress| {
            assert!(progress.report(5, 10));
            42
        });
        assert_eq!(wait(&job), Some(42));
        assert_eq!(job.fraction(), 0.5);
    }

    #[test]
    fn cancellation_stops_the_work() {
        let job = Job::spawn(|progress| {
            let mut steps = 0u64;
            while progress.report(0, 1) {
                steps += 1;
                std::thread::sleep(Duration::from_millis(1));
            }
            steps
        });
        job.cancel();
        assert!(wait(&job).is_some());
    }
}
