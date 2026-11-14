//! Running analyses without freezing the interface.
//!
//! On the desktop a job runs on its own thread. A web page has a single
//! thread, so there a job does a little of its work each time it is
//! polled, for a few hundredths of a second, and the page is redrawn and
//! answers the user in between. For that, work is written as a step that is
//! called again and again until it has a result, and that stops early when
//! told its time is up.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Shared progress of a running job.
#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicUsize,
    total: AtomicUsize,
    cancelled: AtomicBool,
    /// What the job is doing, for a label beside the progress bar.
    stage: Mutex<String>,
}

impl Progress {
    /// Report progress. Returns `false` once the job has been cancelled, at
    /// which point the work should stop.
    pub fn report(&self, done: usize, total: usize) -> bool {
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        !self.cancelled.load(Ordering::Relaxed)
    }

    /// Report what the job is doing and how far it has got with that (a
    /// total of zero when that cannot be told). Returns `false` once the
    /// job has been cancelled.
    pub fn report_stage(&self, stage: &str, done: usize, total: usize) -> bool {
        {
            let mut current = self.stage.lock().unwrap_or_else(|e| e.into_inner());
            if *current != stage {
                current.clear();
                current.push_str(stage);
            }
        }
        self.report(done, total)
    }

    /// What the job said it is doing, if anything.
    pub fn stage(&self) -> String {
        self.stage.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// `(done, total)` of the current stage.
    pub fn counts(&self) -> (usize, usize) {
        (
            self.done.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed),
        )
    }

    /// True once the job has been asked to stop.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
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

/// The time in milliseconds, from some fixed moment: for working in slices
/// of a frame.
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
            * 1000.0
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

    /// The job's progress, with what it is doing.
    pub fn progress(&self) -> &Progress {
        &self.progress
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

        /// Start work made of steps. `step` is called until it returns a
        /// result; its second argument says whether it should return soon,
        /// which on its own thread it never needs to.
        pub fn spawn_steps(
            mut step: impl FnMut(&Progress, &dyn Fn() -> bool) -> Option<T> + Send + 'static,
        ) -> Job<T> {
            Job::spawn(move |progress| loop {
                if let Some(value) = step(progress, &|| false) {
                    return value;
                }
            })
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

    type Step<T> = Box<dyn FnMut(&Progress, &dyn Fn() -> bool) -> Option<T>>;

    /// How long a job works each time it is polled, in milliseconds: short
    /// enough for the page to stay responsive.
    const SLICE_MS: f64 = 25.0;

    pub struct Inner<T> {
        step: RefCell<Option<Step<T>>>,
        announced: Cell<bool>,
    }

    impl<T: 'static> Job<T> {
        /// Queue `work`, to be done in one piece on the page's only thread.
        pub fn spawn(work: impl FnOnce(&Progress) -> T + 'static) -> Job<T> {
            let mut work = Some(work);
            Job::spawn_steps(move |progress, _| work.take().map(|work| work(progress)))
        }

        /// Queue work made of steps. Each poll calls `step` until its time
        /// is up or it returns a result.
        pub fn spawn_steps(
            step: impl FnMut(&Progress, &dyn Fn() -> bool) -> Option<T> + 'static,
        ) -> Job<T> {
            Job {
                inner: Inner {
                    step: RefCell::new(Some(Box::new(step))),
                    announced: Cell::new(false),
                },
                progress: Arc::new(Progress::default()),
            }
        }

        /// The result, once the job has finished. `Err(true)` while there
        /// is more to do; `Err(false)` if it has already been collected.
        pub fn poll(&self) -> Result<T, bool> {
            // Report "running" once first, so a frame is drawn saying so.
            if !self.inner.announced.replace(true) {
                return Err(true);
            }
            let mut slot = self.inner.step.borrow_mut();
            let Some(step) = slot.as_mut() else {
                return Err(false);
            };
            let deadline = js_sys::Date::now() + SLICE_MS;
            let out_of_time = || js_sys::Date::now() >= deadline;
            loop {
                if let Some(value) = step(&self.progress, &out_of_time) {
                    *slot = None;
                    return Ok(value);
                }
                if out_of_time() {
                    return Err(true);
                }
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

    #[test]
    fn work_in_steps_runs_until_it_has_a_result() {
        let mut count = 0;
        let job = Job::spawn_steps(move |progress, out_of_time| {
            count += 1;
            progress.report(count, 5);
            assert!(!out_of_time());
            (count == 5).then_some(count)
        });
        assert_eq!(wait(&job), Some(5));
        assert_eq!(job.fraction(), 1.0);
    }
}
