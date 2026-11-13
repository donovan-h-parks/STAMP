//! Running analyses without freezing the interface.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
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

/// A computation running in the background.
pub struct Job<T> {
    receiver: Receiver<T>,
    progress: Arc<Progress>,
}

impl<T: Send + 'static> Job<T> {
    /// Start `work` on another thread. It is given the job's progress handle
    /// to report to and to learn of cancellation from.
    pub fn spawn(work: impl FnOnce(&Progress) -> T + Send + 'static) -> Job<T> {
        let (sender, receiver) = channel();
        let progress = Arc::new(Progress::default());
        let shared = Arc::clone(&progress);
        std::thread::spawn(move || {
            // The receiver is gone if the job was dropped; that is fine.
            let _ = sender.send(work(&shared));
        });
        Job { receiver, progress }
    }

    /// The result, once the job has finished. `Err(true)` while it is still
    /// running; `Err(false)` if it ended without a result.
    pub fn poll(&self) -> Result<T, bool> {
        match self.receiver.try_recv() {
            Ok(value) => Ok(value),
            Err(TryRecvError::Empty) => Err(true),
            Err(TryRecvError::Disconnected) => Err(false),
        }
    }

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

#[cfg(test)]
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
